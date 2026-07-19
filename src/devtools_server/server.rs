use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    pin::Pin,
    sync::{Arc, OnceLock},
    time::Duration,
};

use bwu_redux::devtools_rpc::{
    ConnectionStatusRequest, ConnectionStatusResponse, SetPauseRequest, SetPauseResponse,
    StateChangeRequest, StateChangeResponse, WatchRequest,
    dev_tools_server::{DevTools, DevToolsServer},
};
use dioxus::logger::tracing::{error, info, warn};
use futures::{Stream, StreamExt as _};
use http::HeaderName;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
#[cfg(not(feature = "mcp"))]
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, ExposeHeaders};
use uuid::Uuid;

use super::watch_hub::WatchHub;
use crate::redux::Action;

/// Environment variable overriding the devtools server listen address.
const LISTEN_ADDR_ENV: &str = "BWU_REDUX_DEVTOOLS_ADDR";

#[derive(Debug)]
pub(crate) struct DevToolsService {
    dispatch_tx: Option<UnboundedSender<Action>>,
    hub: Arc<WatchHub>,
}

impl DevToolsService {
    const fn new(dispatch_tx: Option<UnboundedSender<Action>>, hub: Arc<WatchHub>) -> Self {
        Self { dispatch_tx, hub }
    }
}

#[tonic::async_trait]
impl DevTools for DevToolsService {
    async fn state_change(
        &self,
        request: Request<StateChangeRequest>,
    ) -> Result<Response<StateChangeResponse>, Status> {
        let req: StateChangeRequest = request.into_inner();

        let app_id = req
            .app_id
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("missing app_id"))?;
        let app_id = Uuid::parse_str(app_id.value.as_str())
            .map_err(|err| Status::invalid_argument(format!("invalid app_id UUID: {err}")))?;

        // Ack every received counter regardless of pause filtering, so the
        // monitored app's client sees a normal accept (no error, no retry)
        // for actions that are simply not being forwarded right now.
        let counter = req.changes.iter().map(|v| v.counter).collect::<Vec<_>>();
        let kept = self.hub.publish(app_id, &req);

        if let Some(ref dispatch_tx) = self.dispatch_tx
            && !kept.is_empty()
        {
            let filtered_req = StateChangeRequest {
                changes: kept,
                ..req
            };
            let action = Action::try_from(filtered_req).map_err(Status::invalid_argument)?;
            if let Err(err) = dispatch_tx.send(action) {
                // TODO(zoechi): propagate error
                error!("dispatching action StateUpdate failed: {err}");
                return Err(Status::internal("Processing received changes failed"));
            }
        }

        Ok(Response::new(StateChangeResponse { counter }))
    }

    async fn connection_status(
        &self,
        _request: Request<ConnectionStatusRequest>,
    ) -> Result<Response<ConnectionStatusResponse>, Status> {
        let reply = ConnectionStatusResponse { ok: true };
        Ok(Response::new(reply))
    }

    async fn set_pause(
        &self,
        request: Request<SetPauseRequest>,
    ) -> Result<Response<SetPauseResponse>, Status> {
        let req = request.into_inner();
        let app_id = req
            .app_id
            .as_ref()
            .ok_or_else(|| Status::invalid_argument("missing app_id"))?;
        let app_id = Uuid::parse_str(app_id.value.as_str())
            .map_err(|err| Status::invalid_argument(format!("invalid app_id UUID: {err}")))?;

        self.hub
            .set_pause(app_id, req.paused_action_prefixes.into_iter().collect());

        Ok(Response::new(SetPauseResponse { ok: true }))
    }

    type WatchStream = Pin<Box<dyn Stream<Item = Result<StateChangeRequest, Status>> + Send>>;

    async fn watch(
        &self,
        _request: Request<WatchRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let (replay, rx) = self.hub.subscribe();
        info!(
            "Watch subscriber connected; replaying history of {} app(s)",
            replay.len()
        );

        let live = BroadcastStream::new(rx).filter_map(|item| async move {
            match item {
                Ok(req) => Some(Ok(req)),
                Err(BroadcastStreamRecvError::Lagged(count)) => {
                    warn!("Watch subscriber lagged; {count} state change batches dropped");
                    None
                }
            }
        });
        let stream = futures::stream::iter(replay.into_iter().map(Ok)).chain(live);

        Ok(Response::new(Box::pin(stream)))
    }
}

#[derive(Clone, Debug)]
pub struct DevtoolsServer {
    dispatch_tx: Option<UnboundedSender<Action>>,
    hub: Arc<WatchHub>,
}

const DEFAULT_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);
const DEFAULT_EXPOSED_HEADERS: [HeaderName; 3] = [
    HeaderName::from_static("grpc-status"),
    HeaderName::from_static("grpc-message"),
    HeaderName::from_static("grpc-status-details-bin"),
];

static GRPC_WEB_CORS: OnceLock<CorsLayer> = OnceLock::new();

fn init_grpc_web_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_headers(AllowHeaders::any())
        .allow_methods(AllowMethods::any())
        // .allow_credentials(true)
        .max_age(DEFAULT_MAX_AGE)
        .expose_headers(ExposeHeaders::from(DEFAULT_EXPOSED_HEADERS))
}

fn listen_addr() -> SocketAddr {
    let default_addr = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 49051, 0, 0);
    match std::env::var(LISTEN_ADDR_ENV) {
        Ok(value) => value.parse().unwrap_or_else(|err| {
            warn!("Ignoring invalid {LISTEN_ADDR_ENV}={value}: {err}");
            default_addr.into()
        }),
        Err(_) => default_addr.into(),
    }
}

/// `addr` formatted for humans to paste into a browser or MCP client
/// config: loopback addresses (the common case — see `listen_addr`'s
/// default) read as `localhost:{port}` rather than a bracketed IPv6
/// literal like `[::1]:49051`.
#[cfg(feature = "mcp")]
fn display_host(addr: SocketAddr) -> String {
    if addr.ip().is_loopback() {
        format!("localhost:{}", addr.port())
    } else {
        addr.to_string()
    }
}

#[cfg(feature = "mcp")]
fn log_mcp_status(addr: SocketAddr) {
    info!("MCP interface enabled at http://{}/mcp", display_host(addr));
}

#[cfg(not(feature = "mcp"))]
fn log_mcp_status(_addr: SocketAddr) {
    info!("MCP interface disabled (build with the `mcp` feature to enable it)");
}

impl DevtoolsServer {
    /// With a `dispatch_tx`, received state changes are additionally
    /// dispatched into the local store (embedded desktop GUI usage);
    /// without one, changes are only buffered and re-broadcast to `Watch`
    /// subscribers (standalone server usage).
    pub fn new(dispatch_tx: Option<UnboundedSender<Action>>) -> Self {
        Self {
            dispatch_tx,
            hub: Arc::new(WatchHub::new()),
        }
    }

    /// Returns a copy of `self` with `dispatch_tx` set, sharing the same
    /// hub (and so the same pause state) as `self`. Lets desktop create a
    /// pause-capable handle before the store (and thus its dispatch sender)
    /// exists, then hand the fully-wired server to `run()` once it does.
    #[must_use]
    pub fn with_dispatch_tx(&self, dispatch_tx: UnboundedSender<Action>) -> Self {
        Self {
            dispatch_tx: Some(dispatch_tx),
            hub: Arc::clone(&self.hub),
        }
    }

    /// Direct in-process equivalent of the `SetPause` RPC, for a GUI that
    /// embeds this server (desktop) and so doesn't need to round-trip
    /// through gRPC to talk to itself.
    pub fn set_pause(
        &self,
        app_id: uuid::Uuid,
        paused_action_prefixes: std::collections::HashSet<String>,
    ) {
        self.hub.set_pause(app_id, paused_action_prefixes);
    }

    pub async fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let addr = listen_addr();
        let devtools_service: DevToolsService =
            DevToolsService::new(self.dispatch_tx.clone(), Arc::clone(&self.hub));
        info!("DevtoolsServer listening on {addr}");
        log_mcp_status(addr);
        if let Err(error) = Self::serve(addr, devtools_service, Arc::clone(&self.hub)).await {
            if let Some(ref dispatch_tx) = self.dispatch_tx {
                let _ = dispatch_tx.send(Action::StateUpdateFailure {
                    error: error.to_string(),
                });
            }
            warn!(
                "Running DevtoolsServer failed (a standalone server \u{2014} e.g. the devenv \
                 `redux-devtools` process \u{2014} may already be listening on {addr}): {error:?}"
            );
            Err(Arc::from(error))
        } else {
            info!("DevtoolsServer stopped");
            Ok(())
        }
    }

    #[cfg(not(feature = "mcp"))]
    async fn serve(
        addr: SocketAddr,
        devtools_service: DevToolsService,
        _hub: Arc<WatchHub>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Server::builder()
            .accept_http1(true)
            .layer(GRPC_WEB_CORS.get_or_init(init_grpc_web_cors))
            .layer(GrpcWebLayer::new())
            .add_service(DevToolsServer::new(devtools_service))
            .serve(addr)
            .await
            .map_err(Into::into)
    }

    /// Same gRPC/gRPC-web endpoints as the non-`mcp` build, converted to an
    /// `axum::Router` (`tonic::service::Routes::into_axum_router`) so an MCP
    /// Streamable HTTP service can be mounted at `/mcp` on the same port.
    /// `axum::serve` auto-negotiates HTTP/1.1 (grpc-web, browsers) and h2c
    /// (native gRPC clients) per connection, so this needs no equivalent of
    /// `accept_http1(true)`.
    #[cfg(feature = "mcp")]
    async fn serve(
        addr: SocketAddr,
        devtools_service: DevToolsService,
        hub: Arc<WatchHub>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use rmcp::transport::streamable_http_server::{
            StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
        };

        // Layer order matters: axum makes the *last* `.layer()` outermost,
        // the reverse of tonic's `Server::builder()` in the non-`mcp` path.
        // CORS must be outermost — `GrpcWebLayer` answers any non-grpc-web
        // HTTP/1.1 request (including CORS preflight OPTIONS) with a bare
        // 400, so with CORS inside it browsers never get preflight approval
        // and block every grpc-web call.
        let router = tonic::service::Routes::new(DevToolsServer::new(devtools_service))
            .into_axum_router()
            .layer(GrpcWebLayer::new())
            .layer(GRPC_WEB_CORS.get_or_init(init_grpc_web_cors).clone())
            .route_service(
                "/mcp",
                StreamableHttpService::new(
                    move || Ok(super::mcp::DevToolsMcp::new(Arc::clone(&hub))),
                    Arc::new(LocalSessionManager::default()),
                    StreamableHttpServerConfig::default(),
                ),
            );

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await.map_err(Into::into)
    }
}

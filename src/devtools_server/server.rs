use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    pin::Pin,
    sync::{Arc, OnceLock},
    time::Duration,
};

use bwu_redux::devtools_rpc::{
    ConnectionStatusRequest, ConnectionStatusResponse, StateChangeRequest, StateChangeResponse,
    WatchRequest,
    dev_tools_server::{DevTools, DevToolsServer},
};
use dioxus::logger::tracing::{error, info, warn};
use futures::{Stream, StreamExt as _};
use http::HeaderName;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tonic::{Request, Response, Status, transport::Server};
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

        self.hub.publish(app_id, &req);

        let counter = req.changes.iter().map(|v| v.counter).collect::<Vec<_>>();

        if let Some(ref dispatch_tx) = self.dispatch_tx {
            let action = Action::try_from(req).map_err(Status::invalid_argument)?;
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

    pub async fn run(&self) -> Result<(), Arc<dyn std::error::Error + Send + Sync>> {
        let addr = listen_addr();
        let devtools_service: DevToolsService =
            DevToolsService::new(self.dispatch_tx.clone(), Arc::clone(&self.hub));
        info!("DevtoolsServer listening on {addr}");
        if let Err(error) = Server::builder()
            .accept_http1(true)
            .layer(GRPC_WEB_CORS.get_or_init(init_grpc_web_cors))
            .layer(GrpcWebLayer::new())
            .add_service(DevToolsServer::new(devtools_service))
            .serve(addr)
            .await
        {
            if let Some(ref dispatch_tx) = self.dispatch_tx {
                let _ = dispatch_tx.send(Action::StateUpdateFailure {
                    error: error.to_string(),
                });
            }
            warn!(
                "Running DevtoolsServer failed (a standalone server \u{2014} e.g. the devenv \
                 `redux-devtools` process \u{2014} may already be listening on {addr}): {error:?}"
            );
            Err(Arc::new(error))
        } else {
            info!("DevtoolsServer stopped");
            Ok(())
        }
    }
}

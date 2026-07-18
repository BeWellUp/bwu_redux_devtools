//! Sends `SetPause` requests to a devtools server: directly in-process on
//! desktop (which embeds the server, so no network round-trip is needed —
//! matches the existing architecture where desktop never watches itself
//! over gRPC either), over gRPC-web on the web GUI (which only watches a
//! remote server).

use std::collections::HashSet;

use bwu_redux::async_adapter::spawn_default;
#[cfg(target_family = "wasm")]
use bwu_redux::devtools_rpc::{self, SetPauseRequest, dev_tools_client::DevToolsClient};
#[cfg(not(target_family = "wasm"))]
use bwu_redux_devtools::devtools_server::server::DevtoolsServer;
use bwu_redux_devtools::redux::PauseSink;
#[cfg(target_family = "wasm")]
use tracing::error;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) enum PauseController {
    #[cfg(not(target_family = "wasm"))]
    Embedded(DevtoolsServer),
    #[cfg(target_family = "wasm")]
    Remote,
}

impl PauseController {
    #[cfg(not(target_family = "wasm"))]
    pub(crate) const fn embedded(server: DevtoolsServer) -> Self {
        Self::Embedded(server)
    }

    #[cfg(target_family = "wasm")]
    pub(crate) const fn remote() -> Self {
        Self::Remote
    }

    #[cfg_attr(
        not(target_family = "wasm"),
        expect(
            clippy::unused_async,
            clippy::unused_async_trait_impl,
            reason = "async only to keep one signature shared with the wasm gRPC-web path, which does await"
        )
    )]
    pub(crate) async fn set_pause(&self, app_id: Uuid, paused_action_prefixes: HashSet<String>) {
        match self {
            #[cfg(not(target_family = "wasm"))]
            Self::Embedded(server) => {
                server.set_pause(app_id, paused_action_prefixes);
            }
            #[cfg(target_family = "wasm")]
            Self::Remote => {
                let url = bwu_redux_devtools::devtools_watch::server_url();
                let mut client = DevToolsClient::new(tonic_web_wasm_client::Client::new(url));
                let request = SetPauseRequest {
                    app_id: Some(devtools_rpc::Uuid {
                        value: app_id.to_string(),
                    }),
                    paused_action_prefixes: paused_action_prefixes.into_iter().collect(),
                };
                if let Err(err) = client.set_pause(request).await {
                    error!("SetPause request failed: {err}");
                }
            }
        }
    }
}

impl PauseSink for PauseController {
    /// `PauseMiddleware` calls this synchronously; the actual request (a
    /// direct call on desktop, a gRPC-web round-trip on web) is fired off
    /// on the platform's async runtime rather than blocking the dispatch.
    fn set_pause(&self, app_id: Uuid, paused_action_prefixes: HashSet<String>) {
        let controller = self.clone();
        spawn_default(async move {
            controller.set_pause(app_id, paused_action_prefixes).await;
        });
    }
}

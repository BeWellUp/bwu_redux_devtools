//! MCP interface exposing [`WatchHub`]'s watch/history/pause capabilities,
//! mounted alongside the gRPC/gRPC-web endpoints on the same server (see
//! `server.rs`). Runs in-process, so tool handlers read the hub directly
//! instead of reconstructing state over a gRPC round-trip.

use std::sync::Arc;

use rmcp::{
    ServerHandler, handler::server::wrapper::Parameters, schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use uuid::Uuid;

use super::watch_hub::WatchHub;

#[derive(Clone)]
pub(crate) struct DevToolsMcp {
    hub: Arc<WatchHub>,
}

impl std::fmt::Debug for DevToolsMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevToolsMcp").finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct AppIdParams {
    /// The app's UUID, as shown by `list_apps`.
    pub app_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct GetHistoryParams {
    /// The app's UUID, as shown by `list_apps`.
    pub app_id: String,
    /// Maximum number of most-recent entries to return (default: all buffered).
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub(crate) struct SetPauseParams {
    /// The app's UUID, as shown by `list_apps`.
    pub app_id: String,
    /// Action-name prefixes to stop forwarding for this app; an empty list un-pauses.
    pub paused_action_prefixes: Vec<String>,
}

#[expect(
    clippy::multiple_inherent_impl,
    reason = "#[tool_router] generates a second impl block"
)]
impl DevToolsMcp {
    pub(crate) const fn new(hub: Arc<WatchHub>) -> Self {
        Self { hub }
    }
}

#[tool_router]
impl DevToolsMcp {
    #[tool(description = "List connected apps as `app_id  app_name` pairs.")]
    async fn list_apps(&self) -> Result<String, String> {
        let apps = self.hub.apps();
        if apps.is_empty() {
            return Ok("No apps connected.".to_owned());
        }
        Ok(apps
            .into_iter()
            .map(|(id, name)| format!("{id}  {name}"))
            .collect::<Vec<_>>()
            .join("\n"))
    }

    #[tool(
        description = "Get an app's buffered action/state history (RON-serialized state per entry), most recent last. Optionally limit to the last N entries."
    )]
    async fn get_history(
        &self,
        Parameters(p): Parameters<GetHistoryParams>,
    ) -> Result<String, String> {
        let app_id = Uuid::parse_str(&p.app_id).map_err(|err| format!("Invalid app_id: {err}"))?;
        let history = self
            .hub
            .history(app_id)
            .ok_or_else(|| format!("App {app_id} not found"))?;
        let entries = if let Some(limit) = p.limit {
            let skip = history.len().saturating_sub(limit);
            history.into_iter().skip(skip).collect::<Vec<_>>()
        } else {
            history
        };
        if entries.is_empty() {
            return Ok("No history yet.".to_owned());
        }
        Ok(entries
            .into_iter()
            .map(|change| {
                format!(
                    "[{}] {}\n  state: {}",
                    change.counter, change.action, change.state
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }

    #[tool(description = "Get an app's most recent state (RON-serialized).")]
    async fn get_current_state(
        &self,
        Parameters(p): Parameters<AppIdParams>,
    ) -> Result<String, String> {
        let app_id = Uuid::parse_str(&p.app_id).map_err(|err| format!("Invalid app_id: {err}"))?;
        let history = self
            .hub
            .history(app_id)
            .ok_or_else(|| format!("App {app_id} not found"))?;
        history
            .last()
            .map(|change| change.state.clone())
            .ok_or_else(|| "No history yet.".to_owned())
    }

    #[tool(
        description = "Stop forwarding actions whose name starts with one of the given prefixes for an app; pass an empty list to un-pause."
    )]
    async fn set_pause(&self, Parameters(p): Parameters<SetPauseParams>) -> Result<String, String> {
        let app_id = Uuid::parse_str(&p.app_id).map_err(|err| format!("Invalid app_id: {err}"))?;
        let count = p.paused_action_prefixes.len();
        self.hub
            .set_pause(app_id, p.paused_action_prefixes.into_iter().collect());
        Ok(format!("Paused {count} action prefix(es) for app {app_id}"))
    }
}

#[expect(
    clippy::unused_async_trait_impl,
    reason = "#[tool_handler] generates an async call_tool method that doesn't need to await"
)]
#[tool_handler]
impl ServerHandler for DevToolsMcp {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        use rmcp::model::{ServerCapabilities, ServerInfo};
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "bwu_redux_devtools MCP: inspect connected apps' live Redux state \
             (list_apps, get_history, get_current_state) and pause specific actions \
             (set_pause) without needing the GUI.",
        )
    }
}

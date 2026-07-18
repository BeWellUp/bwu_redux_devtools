use std::collections::HashSet;

use uuid::Uuid;

/// Platform-specific way to tell a devtools server which action prefixes to
/// stop forwarding for an app.
///
/// Implemented outside this crate (the binary's `pause_controller` module)
/// because the concrete mechanism differs desktop (direct in-process call
/// into the embedded server) vs. web (a `SetPause` gRPC-web request) — this
/// trait lets `PauseMiddleware` stay agnostic of which one is active.
pub trait PauseSink: Send + Sync + 'static {
    fn set_pause(&self, app_id: Uuid, paused_action_prefixes: HashSet<String>);
}

/// Used by tests and any context that doesn't wire up a real pause path.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopPauseSink;

impl PauseSink for NoopPauseSink {
    fn set_pause(&self, _app_id: Uuid, _paused_action_prefixes: HashSet<String>) {}
}

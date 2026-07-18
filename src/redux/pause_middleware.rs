use std::sync::Arc;

use bwu_redux::{Middleware, MiddlewareRef, StoreWrapper};

use super::{Action, PauseSink, State};

/// Forwards `PauseActionsChange` to the platform's `PauseSink` (in-process
/// on desktop, gRPC-web `SetPause` on web) before letting the reducer apply
/// it to local UI state. Also runs when `StorageMiddleware` re-dispatches a
/// persisted pause set on an app's first `StateUpdate`, which is how a
/// GUI's stated preference gets re-sent after a devtools server restart
/// (the server's pause state is in-memory only).
#[derive(Clone)]
pub(crate) struct PauseMiddleware {
    sink: Arc<dyn PauseSink>,
}

impl PauseMiddleware {
    pub(crate) const fn new(sink: Arc<dyn PauseSink>) -> Self {
        Self { sink }
    }
}

impl std::fmt::Debug for PauseMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PauseMiddleware").finish_non_exhaustive()
    }
}

impl Middleware<State, Action> for PauseMiddleware {
    fn apply(
        &self,
        store: Arc<StoreWrapper<State, Action>>,
        action: Action,
        next: Arc<MiddlewareRef<State, Action>>,
    ) {
        if let Action::PauseActionsChange {
            app_id,
            ref paused_prefixes,
        } = action
        {
            self.sink.set_pause(
                app_id.into_inner(),
                paused_prefixes.iter().cloned().collect(),
            );
        }

        next.apply(store, action);
    }
}

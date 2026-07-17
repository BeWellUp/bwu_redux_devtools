use bwu_redux_devtools::redux::{
    Action, ChangesStream, StateViewer, Store,
    app_id::AppId,
    selectors::{stream_selected_app_id, stream_selected_app_name, stream_selected_state_viewer},
};

pub(crate) struct AppStateViewFacade {
    store: Store,
}

impl AppStateViewFacade {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub(crate) fn dispatch(&self, action: Action) {
        let _ = self.store.dispatch(action);
    }

    pub(crate) fn get_selected_app_name(&self) -> ChangesStream<Option<String>> {
        stream_selected_app_name(self.store.clone())
    }

    pub(crate) fn get_selected_app_id(&self) -> ChangesStream<Option<AppId>> {
        stream_selected_app_id(self.store.clone())
    }

    pub(crate) fn get_selected_state_viewer(&self) -> ChangesStream<StateViewer> {
        stream_selected_state_viewer(self.store.clone())
    }
}

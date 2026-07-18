use std::sync::Arc;

use bwu_redux_devtools::redux::{
    ChangesStream, GlobalCounter, StateChange, Store,
    app_id::AppId,
    selectors::{
        stream_selected_app_id, stream_selected_change, stream_selected_history_counters_desc,
    },
};

pub(crate) struct StatesListFacade {
    store: Store,
}

impl StatesListFacade {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub(crate) fn get_history_ids(&self) -> ChangesStream<Arc<[GlobalCounter]>> {
        stream_selected_history_counters_desc(self.store.clone())
    }

    pub(crate) fn get_selected_change(&self) -> ChangesStream<Option<StateChange>> {
        stream_selected_change(self.store.clone())
    }

    pub(crate) fn get_selected_app_id(&self) -> ChangesStream<Option<AppId>> {
        stream_selected_app_id(self.store.clone())
    }
}

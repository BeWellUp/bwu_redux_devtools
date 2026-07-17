use std::sync::Arc;

use bwu_redux_devtools::redux::{
    ChangesStream, GlobalCounter, Store, selectors::stream_selected_history_counters_desc,
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
}

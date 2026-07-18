use bwu_redux_devtools::redux::{
    ChangesStream, StateViewer, Store,
    selectors::{
        stream_selected_action_json_pretty, stream_selected_action_prefix,
        stream_selected_action_ron_pretty, stream_selected_action_ron_value,
        stream_selected_previous_state_ron_value, stream_selected_state_json_pretty,
        stream_selected_state_ron_pretty, stream_selected_state_ron_value,
        stream_selected_state_viewer,
    },
};

pub(crate) struct StateExplorerFacade {
    store: Store,
}

impl StateExplorerFacade {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub(crate) fn get_selected_action_prefix(&self) -> ChangesStream<Option<String>> {
        stream_selected_action_prefix(self.store.clone())
    }

    pub(crate) fn get_selected_state_viewer(&self) -> ChangesStream<StateViewer> {
        stream_selected_state_viewer(self.store.clone())
    }

    pub(crate) fn get_selected_action_ron_value(&self) -> ChangesStream<Option<ron::Value>> {
        stream_selected_action_ron_value(self.store.clone())
    }

    pub(crate) fn get_selected_action_ron_pretty(&self) -> ChangesStream<Option<String>> {
        stream_selected_action_ron_pretty(self.store.clone())
    }

    pub(crate) fn get_selected_action_json_pretty(&self) -> ChangesStream<Option<String>> {
        stream_selected_action_json_pretty(self.store.clone())
    }

    pub(crate) fn get_selected_state_ron_value(&self) -> ChangesStream<Option<ron::Value>> {
        stream_selected_state_ron_value(self.store.clone())
    }

    pub(crate) fn get_selected_previous_state_ron_value(
        &self,
    ) -> ChangesStream<Option<ron::Value>> {
        stream_selected_previous_state_ron_value(self.store.clone())
    }

    pub(crate) fn get_selected_state_ron_pretty(&self) -> ChangesStream<Option<String>> {
        stream_selected_state_ron_pretty(self.store.clone())
    }

    pub(crate) fn get_selected_state_json_pretty(&self) -> ChangesStream<Option<String>> {
        stream_selected_state_json_pretty(self.store.clone())
    }
}

use bwu_redux_devtools::redux::{
    GlobalCounter, StateChange, Store,
    selectors::{extract_action_prefix, select_action_for_counter, select_change_for_counter},
};

pub(crate) struct ActionListItemFacade {
    store: Store,
}

impl ActionListItemFacade {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub(crate) fn get_action_prefix(&self, counter: GlobalCounter) -> Option<String> {
        self.store
            .select(select_action_for_counter(counter))
            .map(|s| extract_action_prefix(&s))
    }

    pub(crate) fn get_change(&self, counter: GlobalCounter) -> Option<StateChange> {
        self.store.select(select_change_for_counter(counter))
    }
}

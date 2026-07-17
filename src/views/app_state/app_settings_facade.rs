use bwu_redux_devtools::redux::{
    Action, Store,
    selectors::{stream_selected_drop_history_on_reconnect, stream_selected_history_limit},
};
use dioxus::prelude::*;
use tokio_stream::StreamExt as _;

pub(crate) struct AppSettingsFacade {
    store: Store,
}

impl AppSettingsFacade {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub(crate) fn dispatch(&self, action: Action) {
        let _ = self.store.dispatch(action);
    }

    pub(crate) fn get_history_limit(&self) -> Signal<usize> {
        let mut value = use_signal(|| 0);
        let store = self.store.clone();
        let _ = use_resource(move || {
            let store = store.clone();
            async move {
                let mut stream = stream_selected_history_limit(store);
                while let Some(v) = stream.next().await {
                    value.set(v);
                }
            }
        });
        value
    }

    pub(crate) fn get_drop_history_on_reconnect(&self) -> Signal<bool> {
        let mut value = use_signal(|| false);
        let store = self.store.clone();
        let _ = use_resource(move || {
            let store = store.clone();
            async move {
                let mut stream = stream_selected_drop_history_on_reconnect(store);
                while let Some(v) = stream.next().await {
                    value.set(v);
                }
            }
        });
        value
    }
}

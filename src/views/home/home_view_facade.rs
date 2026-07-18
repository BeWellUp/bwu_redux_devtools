use std::sync::Arc;

use bwu_redux_devtools::redux::{
    Action, Store,
    app_id::AppId,
    selectors::{stream_app_names, stream_selected_app_id},
};
use dioxus::{
    hooks::{use_resource, use_signal},
    prelude::*,
};
use tokio_stream::StreamExt as _;

pub(crate) struct HomeViewFacade {
    store: Store,
}

impl HomeViewFacade {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub(crate) fn dispatch(&self, action: Action) {
        let _ = self.store.dispatch(action);
    }

    pub(crate) fn get_app_names(&self) -> Signal<Vec<(AppId, String)>> {
        let mut app_names: Signal<Vec<(AppId, String)>> = use_signal(|| vec![]);
        let store = Arc::clone(&self.store);
        let _ = use_resource(move || {
            let store = Arc::clone(&store);
            async move {
                let mut stream = stream_app_names(store);

                while let Some(value) = stream.next().await {
                    app_names.set(value);
                }
            }
        });
        app_names
    }

    pub(crate) fn get_selected_app_id(&self) -> Signal<Option<AppId>> {
        let mut selected_app_id: Signal<Option<AppId>> = use_signal(|| None);
        let store = Arc::clone(&self.store);
        let _ = use_resource(move || {
            let store = Arc::clone(&store);
            async move {
                let mut stream = stream_selected_app_id(store);

                while let Some(value) = stream.next().await {
                    selected_app_id.set(value);
                }
            }
        });
        selected_app_id
    }
}

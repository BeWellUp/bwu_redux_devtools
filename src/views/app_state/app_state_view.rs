use bwu_redux_devtools::redux::{Action, StateViewer, Store, app_id::AppId};
use dioxus::prelude::*;
use futures::StreamExt as _;

use super::{AppStateViewFacade, StateExplorer, StatesList};
use crate::{
    components::tabs::{TabList, TabTrigger, Tabs},
    route::Route,
};

const TAB_TREE: &str = "tree";
const TAB_JSON: &str = "json";
const TAB_RON: &str = "ron";

const fn viewer_tab_value(viewer: StateViewer) -> &'static str {
    match viewer {
        StateViewer::Tree => TAB_TREE,
        StateViewer::Json => TAB_JSON,
        StateViewer::Ron => TAB_RON,
    }
}

#[component]
pub fn AppStateView(app_id: String) -> Element {
    let navigator = use_navigator();
    let _full_route = use_route::<Route>();

    let store = use_context::<Store>();
    let facade = use_signal(|| AppStateViewFacade::new(store.clone()));

    let mut selected_app_name: Signal<String> = use_signal(|| String::new());
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_app_name();

        while let Some(value) = stream.next().await {
            selected_app_name.set(value.unwrap_or("App State".into()));
        }
    });

    let mut selected_app_id: Signal<Option<AppId>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_app_id();

        while let Some(value) = stream.next().await {
            selected_app_id.set(value);
        }
    });

    let app_id_clone = app_id.clone();
    let _ = use_effect(move || {
        if let Some(curr_app_id) = selected_app_id() {
            if curr_app_id.to_string() != app_id_clone {
                let _ = navigator.push(Route::AppStateView {
                    app_id: curr_app_id.to_string(),
                });
            }
        }
    });

    let mut state_viewer: Signal<StateViewer> = use_signal(|| StateViewer::default());
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_state_viewer();

        while let Some(value) = stream.next().await {
            state_viewer.set(value);
        }
    });

    rsx! {
        document::Title { "{selected_app_name} - BWU Redux" }

        div { class: "app-state-view", id: "app-state",
            StatesList {}
            div { class: "state-explorer",
                Tabs {
                    value: use_memo(move || Some(viewer_tab_value(state_viewer()).to_owned())),
                    on_value_change: move |value: String| {
                        let viewer = match value.as_str() {
                            TAB_JSON => StateViewer::Json,
                            TAB_RON => StateViewer::Ron,
                            _ => StateViewer::Tree,
                        };
                        facade.read().dispatch(Action::StateViewerChange(viewer));
                    },
                    TabList {
                        TabTrigger { value: TAB_TREE, index: 0_usize, "Tree" }
                        TabTrigger { value: TAB_JSON, index: 1_usize, "JSON" }
                        TabTrigger { value: TAB_RON, index: 2_usize, "Ron" }
                    }
                }
                div { class: "state-explorer-body",
                    StateExplorer {}
                }
            }
        }
    }
}

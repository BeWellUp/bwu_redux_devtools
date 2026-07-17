use std::sync::Arc;

use bwu_redux_devtools::redux::{Action, GlobalCounter, Store, app_id::AppId};
use dioxus::prelude::*;
use futures::StreamExt as _;

use super::{ActionListItemFacade, StatesListFacade};
use crate::components::virtual_list::VirtualList;

#[component]
pub(crate) fn StatesList() -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| StatesListFacade::new(store.clone()));

    let mut items: Signal<Arc<[GlobalCounter]>> = use_signal(|| Default::default());
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_history_ids();

        while let Some(value) = stream.next().await {
            items.set(value);
        }
    });

    // Subscribe this component to `items` so every history change re-renders
    // the list with a fresh `render_item` closure.
    let _ = items();

    let count = use_memo(move || items.read().len());

    rsx! {
        div { class: "states-list",
            div { class: "states-list-inner",
                VirtualList {
                    class: "bwu-list relative overflow-y-auto h-full bg-base-200",
                    count,
                    render_item: move |idx: usize| {
                        let item = items.read().get(idx).copied();
                        match item {
                            Some(item) => rsx! {
                                div { class: "list-row block",
                                    ActionListItem { item }
                                }
                            },
                            None => rsx! {},
                        }
                    },
                }
            }
        }
    }
}

#[derive(Props, Clone, Debug, PartialEq)]
struct ActionListItemProps {
    item: GlobalCounter,
}

#[component]
pub(crate) fn ActionListItem(props: ActionListItemProps) -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| ActionListItemFacade::new(store.clone()));

    let action_prefix = facade.read().get_action_prefix(props.item);
    let change = facade.read().get_change(props.item);

    let mut app_id: Signal<Option<AppId>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_app_id();

        while let Some(value) = stream.next().await {
            app_id.set(value);
        }
    });

    let mut is_selected: Signal<bool> = use_signal(|| false);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_change();

        while let Some(value) = stream.next().await {
            is_selected.set(value.is_some_and(|c| c.counter == props.item));
        }
    });

    if let Some(action_name) = action_prefix
        && let Some(state) = change
        && let Some(app_id) = app_id()
    {
        rsx! {
            div {
                key: "{app_id}-{props.item}",
                class: if is_selected() { "item-active" },
                class: "action-list-item",
                onclick: move |_| {
                    let _ = store
                        .dispatch(Action::SelectedStateChange {
                            counter: props.item,
                        });
                },
                span { class: "action-name", "({state.counter}, {state.session_counter}). {action_name}" }
            }
        }
    } else {
        rsx! {}
    }
}

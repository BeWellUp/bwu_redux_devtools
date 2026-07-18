use std::sync::Arc;

use bwu_redux_devtools::redux::{
    Action, GlobalCounter, Store, app_id::AppId, selectors::select_selected_paused_actions,
};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdFilterX};
use dioxus_primitives::{ContentAlign, ContentSide};
use futures::StreamExt as _;

use super::{ActionListItemFacade, StatesListFacade};
use crate::components::{
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
    virtual_list::VirtualList,
};

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

    // Selection state is subscribed once here and handed to the rows as plain
    // props. Rows must not subscribe per-instance: the virtual list keys rows
    // by index, so a row component outlives any particular item and a stream
    // started inside it would compare against a stale captured counter.
    let mut selected_counter: Signal<Option<GlobalCounter>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_change();

        while let Some(value) = stream.next().await {
            selected_counter.set(value.map(|c| c.counter));
        }
    });

    let mut app_id: Signal<Option<AppId>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_app_id();

        while let Some(value) = stream.next().await {
            app_id.set(value);
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
                        match (item, app_id()) {
                            (Some(item), Some(app_id)) => rsx! {
                                div { class: "list-row block",
                                    ActionListItem {
                                        item,
                                        app_id,
                                        is_selected: selected_counter() == Some(item),
                                    }
                                }
                            },
                            _ => rsx! {},
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
    app_id: AppId,
    is_selected: bool,
}

#[component]
pub(crate) fn ActionListItem(props: ActionListItemProps) -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| ActionListItemFacade::new(store.clone()));

    let action_prefix = facade.read().get_action_prefix(props.item);
    let change = facade.read().get_change(props.item);

    if let Some(action_name) = action_prefix
        && let Some(state) = change
    {
        let app_id = props.app_id;
        rsx! {
            div {
                key: "{app_id}-{props.item}",
                class: if props.is_selected { "item-active" },
                class: "action-list-item",
                onclick: move |_| {
                    let _ = store
                        .dispatch(Action::SelectedStateChange {
                            counter: props.item,
                        });
                },
                div { class: "action-trigger",
                    Tooltip {
                        TooltipTrigger {
                            span { class: "action-counter", "({state.counter}, {state.session_counter})" }
                        }
                        TooltipContent {
                            side: ContentSide::Bottom,
                            align: ContentAlign::Start,
                            class: "nowrap",
                            "Action counter (global, app run)"
                        }
                    }
                    span { class: "action-name", "{action_name}" }
                }
                span { class: "action-spacer" }
                Tooltip {
                    TooltipTrigger {
                        class: "snooze-action-btn",
                        "aria-label": "Pause {action_name}",
                        span {
                            onclick: {
                                let store = store.clone();
                                move |evt: Event<MouseData>| {
                                    evt.stop_propagation();
                                    let mut paused = store.select(select_selected_paused_actions);
                                    let _ = paused.insert(action_name.clone());
                                    let _ = store
                                        .dispatch(Action::PauseActionsChange {
                                            app_id,
                                            paused_prefixes: paused,
                                        });
                                }
                            },
                            Icon { icon: LdFilterX }
                        }
                    }
                    TooltipContent {
                        side: ContentSide::Bottom,
                        align: ContentAlign::End,
                        class: "nowrap",
                        "Pause this action"
                    }
                }
            }
        }
    } else {
        rsx! {}
    }
}

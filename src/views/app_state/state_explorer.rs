use bwu_redux_devtools::redux::{
    StateViewer, Store,
    ron_diff::{self, DiffNode, DiffStatus},
};
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use futures::StreamExt as _;

use super::StateExplorerFacade;
use crate::components::{
    daisyui::{CollapseStyle, Menu, MenuItem, MenuSize, SubMenu},
    icon_tooltip::IconTooltip,
    icons,
};

fn diff_status_class(status: DiffStatus) -> &'static str {
    match status {
        DiffStatus::Unchanged => "",
        DiffStatus::Added => "ron-added",
        DiffStatus::Removed => "ron-removed",
        DiffStatus::Changed => "ron-changed",
    }
}

#[component]
pub(crate) fn StateExplorer() -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| StateExplorerFacade::new(store.clone()));

    let mut action_name: Signal<Option<String>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_action_prefix();

        while let Some(value) = stream.next().await {
            action_name.set(value);
        }
    });

    let mut state_viewer: Signal<StateViewer> = use_signal(|| StateViewer::default());
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_state_viewer();

        while let Some(value) = stream.next().await {
            state_viewer.set(value);
        }
    });

    let mut action_ron_value: Signal<Option<ron::Value>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_action_ron_value();

        while let Some(value) = stream.next().await {
            action_ron_value.set(value);
        }
    });

    let mut state_ron_value: Signal<Option<ron::Value>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_state_ron_value();

        while let Some(value) = stream.next().await {
            state_ron_value.set(value);
        }
    });

    let mut previous_state_ron_value: Signal<Option<ron::Value>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_previous_state_ron_value();

        while let Some(value) = stream.next().await {
            previous_state_ron_value.set(value);
        }
    });

    let mut hide_unchanged: Signal<bool> = use_signal(|| false);

    let state_diff = use_memo(move || {
        state_ron_value().map(|value| ron_diff::diff(previous_state_ron_value().as_ref(), &value))
    });

    let mut action_ron_pretty: Signal<Option<String>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_action_ron_pretty();

        while let Some(value) = stream.next().await {
            action_ron_pretty.set(value);
        }
    });

    let mut state_ron_pretty: Signal<Option<String>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_state_ron_pretty();

        while let Some(value) = stream.next().await {
            state_ron_pretty.set(value);
        }
    });

    let mut action_json_pretty: Signal<Option<String>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_action_json_pretty();

        while let Some(value) = stream.next().await {
            action_json_pretty.set(value);
        }
    });

    let mut state_json_pretty: Signal<Option<String>> = use_signal(|| None);
    let _ = use_resource(move || async move {
        let mut stream = facade.read().get_selected_state_json_pretty();

        while let Some(value) = stream.next().await {
            state_json_pretty.set(value);
        }
    });

    match state_viewer() {
        StateViewer::Tree => {
            if let (Some(action_value), Some(state_value)) = (action_ron_value(), state_ron_value())
            {
                rsx! {
                    Menu {
                        menu_size: MenuSize::XS,
                        MenuItem { is_title: true, "Action" }
                        SubMenu {
                            collapse_style: CollapseStyle::DetailsSummary,
                            title: rsx! {
                                IconTooltip { text: "Map or struct",
                                    Icon { class: "icon", icon: icons::Map {} }
                                }
                                "{action_name().unwrap_or_default()}"
                            },
                            StateValueRon { value: action_value }
                        }
                    }

                    label { class: "label cursor-pointer justify-start gap-x-2 state-diff-toggle",
                        input {
                            r#type: "checkbox",
                            class: "checkbox checkbox-xs",
                            checked: hide_unchanged(),
                            onchange: move |evt| hide_unchanged.set(evt.checked()),
                        }
                        "Hide unchanged"
                    }

                    Menu {
                        menu_size: MenuSize::XS,
                        SubMenu {
                            collapse_style: CollapseStyle::DetailsSummary,
                            title: rsx! {
                                IconTooltip { text: "Map or struct",
                                    Icon { class: "icon", icon: icons::Map {} }
                                }
                                "State"
                                if state_diff().is_some_and(|diff| diff.has_changes()) {
                                    span { class: "ron-changed-indicator", "●" }
                                }
                            },
                            StateValueRon {
                                value: state_value,
                                diff: state_diff(),
                                hide_unchanged: hide_unchanged(),
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
        StateViewer::Json => {
            rsx! {
                pre { class: "state-code-block",
                    "{action_json_pretty().unwrap_or_default()}"
                }
                pre { class: "state-code-block",
                    "{state_json_pretty().unwrap_or_default()}"
                }
            }
        }
        StateViewer::Ron => {
            rsx! {
                pre { class: "state-code-block",
                    "{action_ron_pretty().unwrap_or_default()}"
                }
                pre { class: "state-code-block",
                    "{state_ron_pretty().unwrap_or_default()}"
                }
            }
        }
    }
}

#[component]
pub(crate) fn StateItemValueRon(
    value: ron::Value,
    #[props(default)] diff: Option<DiffNode>,
) -> Element {
    let diff_class = diff
        .as_ref()
        .map(|d| diff_status_class(d.status()))
        .unwrap_or_default();
    let content = match value {
        ron::Value::Bool(v) => {
            if v {
                rsx! {
                    IconTooltip { text: "true",
                        Icon { class: "icon", icon: icons::BoolTrue {} }
                    }
                }
            } else {
                rsx! {
                    IconTooltip { text: "false",
                        Icon { class: "icon", icon: icons::BoolFalse {} }
                    }
                }
            }
        }
        ron::Value::Char(c) => rsx! {
            IconTooltip { text: "Character",
                Icon { class: "icon", icon: icons::Char {} }
            }
            "{c}"
        },
        ron::Value::Map(map) => {
            if map.is_empty() {
                rsx! {
                    IconTooltip { text: "Empty",
                        Icon { class: "icon", icon: icons::MapEmpty {} }
                    }
                }
            } else {
                rsx! {}
            }
        }
        ron::Value::Number(number) => rsx! {
            IconTooltip { text: "Number",
                Icon { class: "icon", icon: icons::Number {} }
            }
            "{number.into_f64()}"
        },
        ron::Value::Option(inner) => match inner {
            Some(inner_value) => rsx! {
                IconTooltip { text: "Optional value (Some)",
                    Icon { class: "icon", icon: icons::Option {} }
                }
                StateItemValueRon { value: inner_value.as_ref().clone(), diff: diff.clone() }
            },
            None => rsx! {
                IconTooltip { text: "No value (None)",
                    Icon { class: "icon", icon: icons::OptionNone {} }
                }
            },
        },
        ron::Value::String(s) => rsx! {
            IconTooltip { text: "String",
                Icon { class: "icon", icon: icons::String {} }
            }
            "{s}"
        },
        ron::Value::Bytes(_items) => rsx! {
            IconTooltip { text: "Byte string",
                Icon { class: "icon", icon: icons::String {} }
            }
            "Bytes"
        },
        ron::Value::Seq(values) => {
            if values.is_empty() {
                rsx! {
                    IconTooltip { text: "Empty",
                        Icon { class: "icon", icon: icons::MapEmpty {} }
                    }
                }
            } else {
                rsx! {}
            }
        }
        ron::Value::Unit => rsx! {
            IconTooltip { text: "Unit value ()",
                Icon { class: "icon", icon: icons::Unit {} }
            }
        },
    };
    rsx! {
        span { class: "ron-item {diff_class}", {content} }
    }
}

#[component]
pub(crate) fn StateValueRon(
    value: ron::Value,
    #[props(default)] diff: Option<DiffNode>,
    #[props(default)] hide_unchanged: bool,
) -> Element {
    if hide_unchanged && diff.as_ref().is_some_and(|d| !d.has_changes()) {
        return rsx! {};
    }

    match value {
        ron::Value::Bool(_)
        | ron::Value::Char(_)
        | ron::Value::Number(_)
        | ron::Value::String(_)
        | ron::Value::Bytes(_)
        | ron::Value::Unit => rsx! {
            MenuItem {
                StateItemValueRon { value: value.clone(), diff }
            }
        },
        ron::Value::Option(_) => {
            if can_render_directly(&value) {
                rsx! {
                    MenuItem {
                        StateItemValueRon { value: value.clone(), diff }
                    }
                }
            } else {
                let ron::Value::Option(inner_opt) = value.clone() else {
                    unreachable!("matched as Option variant above")
                };
                let inner_value = inner_opt.map_or(ron::Value::Unit, |boxed| *boxed);
                rsx! {
                    SubMenu {
                        collapse_style: CollapseStyle::DetailsSummary,
                        title: rsx! {
                            IconTooltip { text: "Optional value (Some)",
                                Icon { class: "icon", icon: icons::Option {} }
                            }
                            if diff.as_ref().is_some_and(DiffNode::has_changes) {
                                span { class: "ron-changed-indicator", "\u{25cf}" }
                            }
                        },
                        StateValueRon { value: inner_value, diff: diff.clone(), hide_unchanged }
                    }
                }
            }
        }
        ron::Value::Map(map) => {
            let removed_keys: Vec<ron::Value> = match &diff {
                Some(DiffNode::Map(_, diff_entries)) => diff_entries
                    .iter()
                    .filter(|(key, child)| {
                        child.status() == DiffStatus::Removed && map.get(key).is_none()
                    })
                    .map(|(key, _)| key.clone())
                    .collect(),
                _ => Vec::new(),
            };

            let items = map.iter().filter_map(|(key, value)| {
                let child_diff = diff.as_ref().and_then(|d| d.map_child(key)).cloned();
                if hide_unchanged && child_diff.as_ref().is_some_and(|d| !d.has_changes()) {
                    return None;
                }

                let key_direct = can_render_directly(key);
                let value_direct = can_render_directly(value);
                let has_child_changes = child_diff.as_ref().is_some_and(DiffNode::has_changes);

                Some(if key_direct && value_direct {
                    rsx! {
                        MenuItem {
                            IconTooltip { text: "Map key",
                                Icon { class: "icon", icon: icons::MapKey {} }
                            }
                            span { class: "map-kv-pair",
                                StateItemValueRon { value: key.clone() }
                                ":"
                                StateItemValueRon { value: value.clone(), diff: child_diff.clone() }
                            }
                        }
                    }
                } else {
                    rsx! {
                        if key_direct {
                            SubMenu {
                                collapse_style: CollapseStyle::DetailsSummary,
                                title: rsx! {
                                    IconTooltip { text: "Map key",
                                        Icon { class: "icon", icon: icons::MapKey {} }
                                    }
                                    span { class: "map-kv-pair",
                                        StateItemValueRon { value: key.clone() }
                                    }
                                    if has_child_changes {
                                        span { class: "ron-changed-indicator", "\u{25cf}" }
                                    }
                                },
                                StateValueRon { value: value.clone(), diff: child_diff.clone(), hide_unchanged }
                            }
                        } else {
                            SubMenu {
                                collapse_style: CollapseStyle::DetailsSummary,
                                title: rsx! {
                                    IconTooltip { text: "Map key",
                                        Icon { class: "icon", icon: icons::MapKey {} }
                                    }
                                },
                                StateValueRon { value: key.clone() }
                            }

                            SubMenu {
                                collapse_style: CollapseStyle::DetailsSummary,
                                title: rsx! {
                                    IconTooltip { text: "Map value",
                                        Icon { class: "icon", icon: icons::MapValue {} }
                                    }
                                    if has_child_changes {
                                        span { class: "ron-changed-indicator", "\u{25cf}" }
                                    }
                                },
                                StateValueRon { value: value.clone(), diff: child_diff.clone(), hide_unchanged }
                            }
                        }
                    }
                })
            });

            let removed_items = removed_keys.into_iter().map(|key| {
                rsx! {
                    MenuItem {
                        span { class: "ron-item ron-removed",
                            IconTooltip { text: "Present in the previous state, removed by this action",
                                Icon { class: "icon", icon: icons::Removed {} }
                            }
                            span { class: "map-kv-pair",
                                StateItemValueRon { value: key.clone() }
                                ":"
                                "(removed)"
                            }
                        }
                    }
                }
            });

            rsx! {
                for item in items {
                    {item}
                }
                for item in removed_items {
                    {item}
                }
            }
        }

        ron::Value::Seq(values) => {
            let removed_count = match &diff {
                Some(DiffNode::Seq(_, diff_items)) if diff_items.len() > values.len() => {
                    diff_items.len() - values.len()
                }
                _ => 0,
            };

            rsx! {
                SubMenu {
                    collapse_style: CollapseStyle::DetailsSummary,
                    title: rsx! {
                        IconTooltip { text: "List or sequence",
                            Icon { class: "icon", icon: icons::Seq {} }
                        }
                        if diff.as_ref().is_some_and(DiffNode::has_changes) {
                            span { class: "ron-changed-indicator", "\u{25cf}" }
                        }
                    },
                    for (index , item) in values.into_iter().enumerate() {
                        StateValueRon {
                            value: item,
                            diff: diff.as_ref().and_then(|d| d.seq_child(index)).cloned(),
                            hide_unchanged,
                        }
                    }
                    for _ in 0..removed_count {
                        MenuItem {
                            span { class: "ron-item ron-removed",
                                IconTooltip { text: "Present in the previous state, removed by this action",
                                    Icon { class: "icon", icon: icons::Removed {} }
                                }
                                "(removed)"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn can_render_directly(value: &ron::Value) -> bool {
    match value {
        ron::Value::Bool(_)
        | ron::Value::Char(_)
        | ron::Value::Number(_)
        | ron::Value::String(_)
        | ron::Value::Bytes(_)
        | ron::Value::Unit => true,
        ron::Value::Map(map) => map.is_empty(),
        ron::Value::Option(value) => {
            value.is_none() || value.as_ref().is_some_and(|v| can_render_directly(v))
        }
        ron::Value::Seq(values) => match values.len() {
            0 => true,
            1 => {
                let value = values
                    .iter()
                    .next()
                    .expect("To always return a value because we just checked length");
                can_render_directly(value)
            }
            _ => false,
        },
    }
}

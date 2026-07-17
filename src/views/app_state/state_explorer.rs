use bwu_redux_devtools::redux::{StateViewer, Store};
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use futures::StreamExt as _;

use super::StateExplorerFacade;
use crate::components::{
    daisyui::{CollapseStyle, Menu, MenuItem, MenuSize, SubMenu},
    icon_tooltip::IconTooltip,
    icons,
};

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

                    Menu {
                        menu_size: MenuSize::XS,
                        SubMenu {
                            collapse_style: CollapseStyle::DetailsSummary,
                            title: rsx! {
                                IconTooltip { text: "Map or struct",
                                    Icon { class: "icon", icon: icons::Map {} }
                                }
                                "State"
                            },
                            StateValueRon { value: state_value }
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
pub(crate) fn StateItemValueRon(value: ron::Value) -> Element {
    match value {
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
        ron::Value::Option(value) => match value {
            Some(value) => {
                rsx! {
                    IconTooltip { text: "Optional value (Some)",
                        Icon { class: "icon", icon: icons::Option {} }
                    }
                    StateItemValueRon { value: value.as_ref().clone() }
                }
            }
            None => {
                rsx! {
                    IconTooltip { text: "No value (None)",
                        Icon { class: "icon", icon: icons::OptionNone {} }
                    }
                }
            }
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
    }
}

#[component]
pub(crate) fn StateValueRon(value: ron::Value) -> Element {
    match value {
        ron::Value::Bool(_)
        | ron::Value::Char(_)
        | ron::Value::Number(_)
        | ron::Value::String(_)
        | ron::Value::Bytes(_)
        | ron::Value::Unit => rsx! {
            MenuItem {
                StateItemValueRon { value: value.clone() }
            }
        },
        ron::Value::Option(_) => {
            if can_render_directly(&value) {
                rsx! {
                    MenuItem {
                        StateItemValueRon { value: value.clone() }
                    }
                }
            } else {
                rsx! {
                    SubMenu {
                        collapse_style: CollapseStyle::DetailsSummary,
                        title: rsx! {
                            IconTooltip { text: "Optional value (Some)",
                                Icon { class: "icon", icon: icons::Option {} }
                            }
                        },
                        StateValueRon { value: value.clone() }
                    }
                }
            }
        }
        ron::Value::Map(map) => {
            let items = map.iter().map(|(key, value)| {
                let key_direct = can_render_directly(key);
                let value_direct = can_render_directly(value);

                if key_direct && value_direct {
                    rsx! {
                        MenuItem {
                            IconTooltip { text: "Map key",
                                Icon { class: "icon", icon: icons::MapKey {} }
                            }
                            span { class: "map-kv-pair",
                                StateItemValueRon { value: key.clone() }
                                ":"
                                StateItemValueRon { value: value.clone() }
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
                                },
                                StateValueRon { value: value.clone() }
                            }
                        } else {
                            SubMenu {
                                collapse_style: CollapseStyle::DetailsSummary,
                                title: rsx! {
                                    IconTooltip { text: "Map key",
                                        Icon { class: "icon", icon: icons::MapKey {} }
                                    }
                                    IconTooltip { text: "Map value",
                                        Icon { class: "icon", icon: icons::MapValue {} }
                                    }
                                },
                                StateValueRon { value: value.clone() }
                            }

                            SubMenu {
                                collapse_style: CollapseStyle::DetailsSummary,
                                title: rsx! {
                                    IconTooltip { text: "Map key",
                                        Icon { class: "icon", icon: icons::MapKey {} }
                                    }
                                    IconTooltip { text: "Map value",
                                        Icon { class: "icon", icon: icons::MapValue {} }
                                    }
                                },
                                StateValueRon { value: value.clone() }
                            }
                        }
                    }
                }
            });

            rsx! {
                for item in items {
                    {item}
                }
            }
        }

        ron::Value::Seq(values) => {
            rsx! {
                SubMenu {
                    collapse_style: CollapseStyle::DetailsSummary,
                    title: rsx! {
                        IconTooltip { text: "List or sequence",
                            Icon { class: "icon", icon: icons::Seq {} }
                        }
                    },
                    for value in values {
                        StateValueRon { value: value.clone() }
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

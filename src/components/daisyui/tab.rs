use std::fmt::Display;

use dioxus::prelude::*;
use tracing::debug;

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct TabListProps {
    #[props(optional)]
    class: String,
    #[props(optional)]
    #[props(default = TabsStyle::Default)]
    tabs_style: TabsStyle,
    #[props(default = TabsSize::Default)]
    size: TabsSize,
    #[props(default = TabsPosition::Default)]
    position: TabsPosition,
    title: Option<String>,
    children: Element,
}

#[component]
pub fn TabList(props: TabListProps) -> Element {
    rsx!(
        div {
            role: "tablist",
            class: "tabs {props.class} {props.tabs_style} {props.size} {props.position}",
            {props.children}
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct TabProps {
    #[props(optional)]
    class: String,

    #[props(optional)]
    #[props(default = false)]
    is_active: bool,
    #[props(optional)]
    #[props(default = false)]
    hover_highlight: bool,
    #[props(optional)]
    onclick: EventHandler<MouseEvent>,
    onfocus: Option<EventHandler<Event<FocusData>>>,
    children: Option<Element>,
}

#[component]
pub fn Tab(props: TabProps) -> Element {
    let tab_active_class = if props.is_active { "tab-active" } else { "" };
    rsx!(
        div {
            role: "tab",
            class: "tab {tab_active_class} {props.class}",
            onclick: move |evt| props.onclick.call(evt),
            {props.children}
        }
    )
}

#[component]
pub fn TabLink(props: TabProps) -> Element {
    let tab_active_class = if props.is_active { "tab-active" } else { "" };
    rsx!(
        a { role: "tab", class: "tab {tab_active_class} {props.class}", {props.children} }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct TabRadioProps {
    label: Option<String>,
    name: String,
    #[props(optional)]
    class: String,

    #[props(optional)]
    #[props(default = false)]
    is_checked: bool,
    #[props(optional)]
    #[props(default = false)]
    hover_highlight: bool,
    onclick: Option<EventHandler<MouseEvent>>,
    onfocus: Option<EventHandler<Event<FocusData>>>,
    children: Option<Element>,
}

#[component]
pub fn TabRadio(props: TabRadioProps) -> Element {
    if props.label.is_none() && props.children.is_none() {
        debug!("One of label or children must be present");
    }
    rsx!(
        if props.children.is_some() {
            label { class: "tab {props.class}",
                input {
                    r#type: "radio",
                    checked: props.is_checked,
                    name: props.name,
                }
                {props.children}
            }
        } else {
            input {
                r#type: "radio",
                checked: props.is_checked,
                name: props.name,
                aria_label: props.label,
                class: "tab {props.class}",
                {props.children}
            }
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct TabContentProps {
    #[props(optional)]
    class: String,
    children: Element,
}

#[component]
pub fn TabContent(props: TabContentProps) -> Element {
    rsx!(
        div { class: "tab-content {props.class}", {props.children} }
    )
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub enum TabsStyle {
    #[default]
    Default,
    Box,
    Border,
    Lift,
}

// impl IntoAttributeValue<String> for TabsStyle {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<TabsStyle> for String {
    #[inline]
    fn from(value: TabsStyle) -> Self {
        match value {
            TabsStyle::Default => Self::new(),
            TabsStyle::Box => Self::from("tabs-box"),
            TabsStyle::Border => Self::from("tabs-border"),
            TabsStyle::Lift => Self::from("tabs-lift"),
        }
    }
}

impl Display for TabsStyle {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub enum TabsSize {
    #[default]
    Default,
    XS,
    SM,
    MD,
    LG,
    XL,
}

// impl IntoAttributeValue<String> for TabsSize {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<TabsSize> for String {
    #[inline]
    fn from(value: TabsSize) -> Self {
        match value {
            TabsSize::Default => Self::new(),
            TabsSize::XS => Self::from("tabs-xs"),
            TabsSize::SM => Self::from("tabs-sm"),
            TabsSize::MD => Self::from("tabs-md"),
            TabsSize::LG => Self::from("tabs-lg"),
            TabsSize::XL => Self::from("tabs-xl"),
        }
    }
}

impl Display for TabsSize {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub enum TabsPosition {
    #[default]
    Default,
    Top,
    Bottom,
}

// impl IntoAttributeValue<String> for TabsPosition {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<TabsPosition> for String {
    #[inline]
    fn from(value: TabsPosition) -> Self {
        match value {
            TabsPosition::Default => Self::new(),
            TabsPosition::Top => Self::from("tabs-top"),
            TabsPosition::Bottom => Self::from("tabs-bottom"),
        }
    }
}

impl Display for TabsPosition {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

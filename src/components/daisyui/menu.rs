use std::fmt::Display;

use dioxus::prelude::*;

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct MenuProps {
    #[props(optional)]
    class: String,
    #[props(default = false)]
    is_dropdown: bool,
    #[props(into, default = MenuSize::Default)]
    menu_size: MenuSize,
    #[props(into, default = MenuDirection::Default)]
    menu_direction: MenuDirection,
    children: Element,
}

#[component]
pub fn Menu(props: MenuProps) -> Element {
    let dropdown_class = if props.is_dropdown {
        "menu-dropdown"
    } else {
        ""
    };
    rsx!(
        ul { class: "menu {props.class} {props.menu_size} {props.menu_direction} {dropdown_class}",
            {props.children}
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct SubMenuProps {
    #[props(optional)]
    class: String,
    // #[props(default = false)]
    // is_dropdown: bool,
    #[props(into, default = MenuSize::Default)]
    menu_size: MenuSize,
    // #[props(into, default = MenuDirection::Default)]
    // menu_direction: MenuDirection,
    title: Option<Element>,
    #[props(default = CollapseStyle::None)]
    collapse_style: CollapseStyle,
    #[props(default = true)]
    is_collapsed: bool,
    children: Element,
}

#[component]
pub fn SubMenu(props: SubMenuProps) -> Element {
    // let dropdown_class = if props.is_dropdown {
    //     "menu-dropdown"
    // } else {
    //     ""
    // };

    match props.collapse_style {
        CollapseStyle::None => rsx! {
            li {
                {props.title}
                ul { class: "{props.class} {props.menu_size}", {props.children} }
            }
        },
        CollapseStyle::DetailsSummary => rsx! {
            li {
                details { open: true,
                    summary { {props.title} }
                    ul { class: "{props.class} {props.menu_size}", {props.children} }
                }
            }
        },
    }
}

#[component]
pub fn DrawerMenu(props: MenuProps) -> Element {
    rsx!(
        ul { class: "menu {props.menu_size} {props.class}", {props.children} }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct MenuItemProps {
    #[props(optional)]
    class: String,
    #[props(default = false)]
    is_title: bool,
    #[props(default = true)]
    is_enabled: bool,
    #[props(default = false)]
    is_active: bool,
    #[props(default = false)]
    is_focused: bool,
    #[props(optional)]
    onclick: EventHandler<Event<MouseData>>,
    children: Element,
}

#[component]
pub fn MenuItem(props: MenuItemProps) -> Element {
    let title_class = if props.is_title { "menu-title" } else { "" };
    let disabled_class = if props.is_enabled {
        ""
    } else {
        "menu-disabled"
    };
    let active_class = if props.is_active { "menu-active" } else { "" };
    let focused_class = if props.is_active { "menu-focus" } else { "" };
    rsx!(
        li {
            class: "menu-item {disabled_class} {title_class}  {props.class}",
            onclick: move |evt| props.onclick.call(evt),
            a { class: "{active_class} {focused_class}", {props.children} }
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct MenuItemTriggerProps {
    r#for: String,
    #[props(optional)]
    class: String,
    #[props(default = true)]
    enabled: bool,
    #[props(optional)]
    onclick: EventHandler<Event<MouseData>>,
    accesskey: Option<String>,
    children: Element,
}

#[component]
pub fn MenuItemTrigger(props: MenuItemTriggerProps) -> Element {
    let disabled_class = if props.enabled { "" } else { "menu-disabled" };
    rsx!(
        li {
            class: "menu-item {disabled_class} {props.class}",
            "data-tooltip": "Toggle sidebar ()",
            label {
                r#for: props.r#for,
                accesskey: props.accesskey,
                onclick: move |evt| props.onclick.call(evt),

                a { {props.children} }
            }
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct RouteMenuItemProps {
    #[props(default = false)]
    is_active: bool,
    #[props(optional)]
    class: String,
    #[props(default = Signal::new(true))]
    is_enabled: Signal<bool>,
    #[props(optional)]
    onclick: EventHandler<Event<MouseData>>,

    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,

    children: Element,
}

/// `MenuItem` that takes a `NavigationTarget`
///
/// When there is a way to pass [`dioxus_router::navigation::NavigationTarget`] as optional argument
// and a way to recognize whether it was passed, it can be merged with [`MenuItem`]
#[component]
pub fn RouteMenuItem(props: RouteMenuItemProps) -> Element {
    // let route: Element;
    let disabled_class = if *props.is_enabled.read() {
        ""
    } else {
        "menu-disabled"
    };
    // let is_active = route == props.to;
    let active_class = if props.is_active { "menu-active" } else { "" };
    rsx!(
        li {
            onclick: move |evt| props.onclick.call(evt),
            class: "menu-item route-menu-item {disabled_class} {props.class} flex {active_class}",
            ..props.attributes,
            // Link { to: props.to, {props.children} }
            {props.children}
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct MenuItemRadioProps {
    label: String,
    /// The value this individual item represents
    value: String,
    /// The group of radio inputs this item belongs to
    name: String,
    #[props(optional, default = false)]
    checked: bool,
    /// Extra classes to add
    #[props(optional)]
    class: String,
    #[props(default = true)]
    enabled: bool,
    #[props(optional)]
    oninput: EventHandler<Event<FormData>>,
    children: Element,
}

#[component]
pub fn MenuItemRadio(props: MenuItemRadioProps) -> Element {
    rsx!(
        li {
            input {
                r#type: "radio",
                name: props.name,
                class: "w-full btn btn-sm btn-block btn-ghost justify-start {props.class}",
                aria_label: props.value.clone(),
                value: props.value,
                checked: props.checked,
                oninput: props.oninput,
                disabled: !props.enabled,
                {props.children}
            }
                // class: if props.enabled { "" } else { "menu-disabled" },
        // a { {props.children} }
        }
    )
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct MenuTitleProps {
    #[props(default = true)]
    enabled: bool,
    #[props(optional)]
    class: String,

    children: Element,
}

#[component]
pub fn MenuTitle(props: MenuTitleProps) -> Element {
    rsx!(
        li { class: "menu-title {props.class}", {props.children} }
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum MenuSize {
    #[default]
    Default,
    XS,
    SM,
    MD,
    LG,
    XL,
}

// impl IntoAttributeValue<String> for MenuSize {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<MenuSize> for String {
    #[inline]
    fn from(value: MenuSize) -> Self {
        match value {
            MenuSize::Default => Self::new(),
            MenuSize::XS => Self::from("menu-xs"),
            MenuSize::SM => Self::from("menu-sm"),
            MenuSize::MD => Self::from("menu-md"),
            MenuSize::LG => Self::from("menu-lg"),
            MenuSize::XL => Self::from("menu-xl"),
        }
    }
}

impl Display for MenuSize {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub enum MenuDirection {
    #[default]
    Default,
    Horizontal,
    Vertical,
}

// impl IntoAttributeValue<String> for MenuDirection {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<MenuDirection> for String {
    #[inline]
    fn from(value: MenuDirection) -> Self {
        match value {
            MenuDirection::Default => Self::new(),
            MenuDirection::Horizontal => Self::from("menu-horizontal"),
            MenuDirection::Vertical => Self::from("menu-vertical"),
        }
    }
}

impl Display for MenuDirection {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub(crate) enum CollapseStyle {
    #[default]
    None,
    DetailsSummary,
}

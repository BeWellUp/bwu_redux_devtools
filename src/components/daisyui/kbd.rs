use std::fmt::Display;

use dioxus::prelude::*;

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct KbdProps {
    #[props(optional)]
    class: String,
    #[props(optional)]
    #[props(into, default = KbdSize::Default)]
    kbd_size: KbdSize,
    children: Element,
}

#[component]
pub fn Kbd(props: KbdProps) -> Element {
    rsx! {
        div { class: "kbd {props.class} {props.kbd_size}", {props.children} }
    }
}

#[derive(Clone, Copy, Default, Debug, Eq, PartialEq)]
pub enum KbdSize {
    #[default]
    Default,
    XS,
    SM,
    MD,
    LG,
    XL,
}

// impl IntoAttributeValue<String> for KbdSize {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<KbdSize> for String {
    #[inline]
    fn from(value: KbdSize) -> Self {
        match value {
            KbdSize::Default => Self::new(),
            KbdSize::XS => Self::from("kbd-xs"),
            KbdSize::SM => Self::from("kbd-sm"),
            KbdSize::MD => Self::from("kbd-md"),
            KbdSize::LG => Self::from("kbd-lg"),
            KbdSize::XL => Self::from("kbd-xl"),
        }
    }
}

impl Display for KbdSize {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardKey {
    Command,
    CommandShort,
    Alt,
    AltShort,
    Shift,
    ShiftShort,
    Control,
    ControlShort,
}

impl From<KeyboardKey> for String {
    #[inline]
    #[expect(clippy::non_ascii_literal, reason = "These should be fine")]
    fn from(value: KeyboardKey) -> Self {
        match value {
            KeyboardKey::Command => Self::from("Cmd"),
            KeyboardKey::CommandShort => Self::from("⌘"),
            KeyboardKey::Alt => Self::from("Alt"),
            KeyboardKey::AltShort => Self::from("⌥"),
            KeyboardKey::Shift => Self::from("Shift"),
            KeyboardKey::ShiftShort => Self::from("⇧"),
            KeyboardKey::Control => Self::from("Ctrl"),
            KeyboardKey::ControlShort => Self::from("⌃"),
        }
    }
}

impl Display for KeyboardKey {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

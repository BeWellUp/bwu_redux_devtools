use dioxus::prelude::*;
use dioxus_primitives::ContentSide;

use crate::components::tooltip::{Tooltip, TooltipContent, TooltipTrigger};

#[derive(Props, Clone, PartialEq)]
pub(crate) struct IconTooltipProps {
    /// Brief explanation of what the wrapped icon represents.
    text: &'static str,
    children: Element,
}

/// Wraps an icon with a tooltip explaining what it represents — used for the
/// RON-value-kind icons in the state tree (`None`, unit, map key/value, …).
#[component]
pub(crate) fn IconTooltip(props: IconTooltipProps) -> Element {
    rsx! {
        Tooltip {
            TooltipTrigger { {props.children} }
            TooltipContent { side: ContentSide::Top, class: "nowrap", "{props.text}" }
        }
    }
}

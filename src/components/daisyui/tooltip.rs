use std::{collections::HashMap, fmt::Display, rc::Rc};

use dioxus::{core::use_drop, html::geometry::PixelsRect, prelude::*};
use tracing::{debug, error, warn};

use crate::components::hooks::{use_id_or, use_unique_id};

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct TooltipProps {
    id: ReadSignal<Option<String>>,
    #[props(optional)]
    class: String,
    #[props(into, default = TooltipPlacement::Default)]
    tooltip_placement: TooltipPlacement,
    #[props(into, default = TooltipColor::Default)]
    tooltip_color: TooltipColor,
    #[props(default = false)]
    is_open: bool,
    #[props(default = false)]
    use_portal: bool,
    tooltip_text: Option<String>,
    tooltip_content: Option<Element>,
    children: Element,
}

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub(crate) struct TooltipPortalCtx {
    pub tooltips: HashMap<String, Element>,
}

#[component]
pub fn TooltipPortal(/*props: TooltipProps*/) -> Element {
    let context = use_context::<Signal<TooltipPortalCtx>>();
    rsx! {
        for tooltip in context.read().tooltips.values() {
            {tooltip}
        }
    }
}

#[component]
#[expect(
    clippy::items_after_statements,
    reason = "To keep the elements close to where they are used"
)]
pub fn Tooltip(props: TooltipProps) -> Element {
    if props.tooltip_text.is_some() && props.tooltip_content.is_some() {
        error!("Either tooltip_text or tooltip_content can be set, but not both.");
    }

    let portal = try_use_context::<Signal<TooltipPortalCtx>>();

    if let Some(mut portal) = portal
        && props.use_portal
    {
        let gen_id = use_unique_id();
        let tooltip_id = use_id_or(gen_id, props.id);

        let mut self_dimensions = use_signal::<Option<PixelsRect>>(|| None);
        let mut self_element = use_signal::<Option<Rc<MountedData>>>(|| None);

        let props_effect = props.clone();
        let props_enter = props.clone();
        let props_resize = props.clone();

        fn create_tooltip_element(id: &str, props: TooltipProps, dimension: PixelsRect) -> Element {
            rsx! {
                div {
                    key: "{id}",
                    class: "tooltip tooltip-open {props.class} {props.tooltip_placement} {props.tooltip_color}",
                    style: "position: fixed; top: {dimension.origin.y}px; left: {dimension.origin.x}px; width: {dimension.size.width}px; height: {dimension.size.height}px; z-index: 11;",
                    "data-tip": props.tooltip_text,
                    {props.tooltip_content.clone()}
                }
            }
        }

        #[expect(unused_results, reason = "We don't need the result")]
        use_effect(move || {
            if props_effect.is_open {
                let mut context = use_context::<Signal<TooltipPortalCtx>>();
                let props_clone = props_effect.clone();
                let id = (*tooltip_id.read()).clone();

                #[expect(unused_results, reason = "We don't need the result")]
                spawn(async move {
                    if let Some(element) = self_element.read().as_deref() {
                        let dim = (*element).get_client_rect().await;
                        if let Ok(dim) = dim {
                            #[expect(unused_results, reason = "Only called for the side effect")]
                            context.with_mut(|v| {
                                v.tooltips.insert(
                                    id.clone(),
                                    create_tooltip_element(&id, props_clone.clone(), dim),
                                )
                            });
                        }
                    }
                });
            }
        });

        use_drop(move || {
            #[expect(unused_results, reason = "Only called for the side effect")]
            portal.with_mut(|v| v.tooltips.remove(&*tooltip_id.read()));
        });

        rsx! {
            div {
                class: "tooltip",
                onmouseenter: move |_cx| {
                    let props_clone = props_enter.clone();
                    #[expect(unused_results, reason = "Only called for the side effect")]
                    spawn(async move {
                        if let Some(element) = self_element.read().as_deref()
                            && let Ok(dim) = (*element).get_client_rect().await
                        {
                            self_dimensions.set(Some(dim));
                            let id = (*tooltip_id.read()).clone();
                            #[expect(unused_results, reason = "Only called for the side effect")]
                            portal
                                .with_mut(|v| {
                                    v.tooltips
                                        .insert(
                                            id.clone(),
                                            create_tooltip_element(&id, props_clone.clone(), dim),
                                        )
                                });
                        }
                    });
                },
                onmouseleave: move |_| {
                    if !props.is_open {
                        #[expect(unused_results, reason = "Only called for the side effect")]
                        portal.with_mut(|v| v.tooltips.remove(&*tooltip_id.read()));
                    }
                },
                onmounted: move |element| {
                    self_element.set(Some(element.data()));
                    #[expect(unused_results, reason = "We don't need the result")]
                    spawn(async move {
                        match element.get_client_rect().await {
                            Ok(dim) => {
                                self_dimensions.set(Some(dim));
                            }
                            Err(err) => {
                                warn!("Couldn't get button dimensions ({err})");
                            }
                        }
                    });
                },
                onresize: move |cx| {
                    cx.stop_propagation();
                    let props_clone = props_resize.clone();
                    #[expect(unused_results, reason = "We don't need the result")]
                    spawn(async move {
                        if let Some(element) = self_element.read().as_deref()
                            && let Ok(dim) = (*element).get_client_rect().await
                            && let Ok(pos) = (*element).get_scroll_offset().await
                            && self_dimensions.read().is_some_and(|d| d != dim)
                        {
                            debug!("scroll: {:?}", pos);
                            self_dimensions.set(Some(dim));
                            let id = (*tooltip_id.read()).clone();
                            if portal.read().tooltips.contains_key(&id) {
                                #[expect(unused_results, reason = "Only called for the side effect")]
                                portal
                                    .with_mut(|v| {
                                        v.tooltips
                                            .insert(
                                                id.clone(),
                                                create_tooltip_element(&id, props_clone.clone(), dim),
                                            )
                                    });
                            }
                        }
                    });
                },
                {props.children}
            }
        }
    } else {
        let is_open_class = if props.is_open { "tooltip-open" } else { "" };
        rsx! {
            div {
                class: "tooltip {props.class}{props.tooltip_placement} {props.tooltip_color} {is_open_class}",
                "data-tip": props.tooltip_text,
                {props.tooltip_content}
                {props.children}
            }
        }
    }
}

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct TooltipContentProps {
    #[props(optional)]
    class: String,
    children: Element,
}

#[component]
pub fn TooltipContent(props: TooltipContentProps) -> Element {
    rsx! {
        div { class: "tooltip-content {props.class}", {props.children} }
    }
}

#[derive(Clone, Copy, Eq, Default, Debug, PartialEq)]
pub enum TooltipPlacement {
    #[default]
    Default,
    Top,
    Bottom,
    Left,
    Right,
}

// impl IntoAttributeValue<String> for TooltipPlacement {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<TooltipPlacement> for String {
    #[inline]
    fn from(value: TooltipPlacement) -> Self {
        match value {
            TooltipPlacement::Default => Self::new(),
            TooltipPlacement::Top => Self::from("tooltip-top"),
            TooltipPlacement::Bottom => Self::from("tooltip-bottom"),
            TooltipPlacement::Left => Self::from("tooltip-left"),
            TooltipPlacement::Right => Self::from("tooltip-right"),
        }
    }
}

impl Display for TooltipPlacement {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Default, Debug)]
pub enum TooltipColor {
    #[default]
    Default,
    Neutral,
    Primary,
    Secondary,
    Accent,
    Info,
    Success,
    Warning,
    Error,
}

// impl IntoAttributeValue<String> for TooltipColor {
//     fn into_value(self) -> dioxus_core::AttributeValue {
//         AttributeValue::Text(String::from(self))
//     }
// }

impl From<TooltipColor> for String {
    #[inline]
    fn from(value: TooltipColor) -> Self {
        match value {
            TooltipColor::Default => Self::new(),
            TooltipColor::Neutral => Self::from("tooltip-neutral"),
            TooltipColor::Primary => Self::from("tooltip-primary"),
            TooltipColor::Secondary => Self::from("tooltip-secondary"),
            TooltipColor::Accent => Self::from("tooltip-accent"),
            TooltipColor::Info => Self::from("tooltip-info"),
            TooltipColor::Success => Self::from("tooltip-success"),
            TooltipColor::Warning => Self::from("tooltip-warning"),
            TooltipColor::Error => Self::from("tooltip-error"),
        }
    }
}

impl Display for TooltipColor {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Into::<String>::into(*self))
    }
}

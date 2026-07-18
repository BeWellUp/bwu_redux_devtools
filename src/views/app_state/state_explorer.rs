#[cfg(not(feature = "web"))]
use std::time::Duration;

use bwu_redux_devtools::redux::{
    StateViewer, Store,
    ron_diff::{self, DiffNode, DiffStatus},
};
use dioxus::prelude::*;
use dioxus_free_icons::Icon;
use futures::StreamExt as _;
#[cfg(not(feature = "web"))]
use futures_timer::Delay;

use super::StateExplorerFacade;
use crate::components::{
    daisyui::{CollapseStyle, Menu, MenuItem, MenuSize, SubMenu},
    icon_tooltip::IconTooltip,
    icons,
};

/// Waits for the browser to have actually painted whatever DOM/style
/// changes are currently pending, before returning.
///
/// A bare `setTimeout`/[`Delay`] offers no such guarantee: it can resolve
/// before the browser's rendering pipeline (style → layout → paint →
/// compositor commit) has run even once, especially for a 0ms delay. That
/// matters here because [`StateExplorer`] relies on the spinner overlay's
/// CSS opacity transition being committed to the compositor *before* the
/// heavy synchronous tree render blocks the main thread — only a
/// compositor-driven transition keeps progressing while JS is blocked.
/// Waiting for two consecutive animation frames is the standard technique
/// to guarantee a real paint happened: the first frame confirms the
/// browser is about to render pending changes, the second only fires once
/// that render has completed.
#[cfg(feature = "web")]
#[allow(
    clippy::future_not_send,
    reason = "wasm32 is single-threaded; gloo_render::AnimationFrame held across the await is fine"
)]
async fn yield_for_paint() {
    for _ in 0..2 {
        let (tx, rx) = futures::channel::oneshot::channel();
        let frame = gloo_render::request_animation_frame(move |_time| {
            let _ = tx.send(());
        });
        let _ = rx.await;
        drop(frame);
    }
}

/// Desktop has no browser rendering pipeline to synchronize with here (no
/// `web_sys::window`); fall back to the same single-tick yield used
/// elsewhere in this crate.
#[cfg(not(feature = "web"))]
async fn yield_for_paint() {
    Delay::new(Duration::from_millis(0)).await;
}

fn diff_status_class(status: DiffStatus) -> &'static str {
    match status {
        DiffStatus::Unchanged => "",
        DiffStatus::Added => "ron-added",
        DiffStatus::Removed => "ron-removed",
        DiffStatus::Changed => "ron-changed",
    }
}

/// Everything the (potentially expensive) viewer render depends on, bundled
/// so it can be snapshotted as one unit and compared by equality. See
/// [`StateExplorer`] for how this is used to delay showing the heavy tree
/// by one frame behind a spinner.
#[derive(Clone, Default, PartialEq)]
struct ExplorerContent {
    viewer: StateViewer,
    action_name: Option<String>,
    action_ron_value: Option<ron::Value>,
    state_ron_value: Option<ron::Value>,
    previous_state_ron_value: Option<ron::Value>,
    hide_unchanged: bool,
    action_json_pretty: Option<String>,
    state_json_pretty: Option<String>,
    action_ron_pretty: Option<String>,
    state_ron_pretty: Option<String>,
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

    // Bundles every input the (potentially very expensive) viewer render
    // below depends on. `rendered_content` lags one frame behind `content`
    // (see the resource below) so the spinner overlay actually gets painted
    // before the heavy synchronous render starts; `ExplorerViewer` takes the
    // snapshot as a prop so it re-renders only when the content itself
    // changes, not on every `is_rendering` toggle.
    let content = use_memo(move || ExplorerContent {
        viewer: state_viewer(),
        action_name: action_name(),
        action_ron_value: action_ron_value(),
        state_ron_value: state_ron_value(),
        previous_state_ron_value: previous_state_ron_value(),
        hide_unchanged: hide_unchanged(),
        action_json_pretty: action_json_pretty(),
        state_json_pretty: state_json_pretty(),
        action_ron_pretty: action_ron_pretty(),
        state_ron_pretty: state_ron_pretty(),
    });

    let mut rendered_content = use_signal(ExplorerContent::default);
    let mut is_rendering = use_signal(|| false);

    let _ = use_resource(move || {
        let next = content();
        async move {
            // Yield before the *first* `is_rendering.set(true)` too: on a
            // fresh mount (e.g. switching back from the Settings tab, which
            // unmounts this whole component), setting it true in the same
            // synchronous pass as the initial mount means the overlay's
            // very first painted frame already has the `active` class —
            // CSS transitions don't animate a value that never had a prior
            // painted frame to transition from, so it would appear
            // instantly instead of after the delay.
            yield_for_paint().await;
            is_rendering.set(true);
            // Yield again so the browser paints the spinner overlay
            // (invisible until CSS's `transition-delay` elapses) *before*
            // the heavy, synchronous tree render below blocks the main
            // thread. A JS/Rust timer can't fire "during" that block since
            // this is single-threaded, but a composited CSS opacity
            // transition keeps ticking regardless — see `.render-spinner-overlay`
            // in input.css.
            yield_for_paint().await;
            rendered_content.set(next);
            // And yield once more before clearing the flag: `set` only
            // marks `ExplorerViewer` dirty, it doesn't run its (expensive)
            // render synchronously. Without this yield, both this write and
            // the one above land in the same reconciliation pass, so the
            // heavy render happens *after* is_rendering is already false
            // and the spinner never gets a chance to stay up during it.
            yield_for_paint().await;
            is_rendering.set(false);
        }
    });

    rsx! {
        div { class: "render-spinner-anchor",
            div {
                class: if is_rendering() { "render-spinner-overlay active" } else { "render-spinner-overlay" },
                span { class: "loading loading-spinner loading-lg" }
            }
        }
        div { class: "state-explorer-render-area",
            ExplorerViewer {
                content: rendered_content(),
                on_hide_unchanged_change: move |value| hide_unchanged.set(value),
            }
        }
    }
}

#[component]
fn ExplorerViewer(content: ExplorerContent, on_hide_unchanged_change: Callback<bool>) -> Element {
    match content.viewer {
        StateViewer::Tree => {
            if let (Some(action_value), Some(state_value)) =
                (content.action_ron_value, content.state_ron_value)
            {
                let state_diff =
                    ron_diff::diff(content.previous_state_ron_value.as_ref(), &state_value);

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
                                "{content.action_name.clone().unwrap_or_default()}"
                            },
                            StateValueRon { value: action_value }
                        }
                    }

                    label { class: "label cursor-pointer justify-start gap-x-2 state-diff-toggle",
                        input {
                            r#type: "checkbox",
                            class: "checkbox checkbox-xs",
                            checked: content.hide_unchanged,
                            onchange: move |evt| on_hide_unchanged_change.call(evt.checked()),
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
                                if state_diff.has_changes() {
                                    span { class: "ron-changed-indicator", "●" }
                                }
                            },
                            StateValueRon {
                                value: state_value,
                                diff: Some(state_diff),
                                hide_unchanged: content.hide_unchanged,
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
                    "{content.action_json_pretty.clone().unwrap_or_default()}"
                }
                pre { class: "state-code-block",
                    "{content.state_json_pretty.clone().unwrap_or_default()}"
                }
            }
        }
        StateViewer::Ron => {
            rsx! {
                pre { class: "state-code-block",
                    "{content.action_ron_pretty.clone().unwrap_or_default()}"
                }
                pre { class: "state-code-block",
                    "{content.state_ron_pretty.clone().unwrap_or_default()}"
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

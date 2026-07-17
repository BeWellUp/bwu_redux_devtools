#![allow(
    unpredictable_function_pointer_comparisons,
    reason = "Component<T> props contain fn pointers; interim until replaced by the official virtual_list"
)]

use std::{rc::Rc, sync::Arc};

use dioxus::prelude::*;
#[cfg(feature = "web")]
use dioxus::web::WebEventExt as _;
use tracing::warn;

use crate::components::{
    focus_provider::FocusProvider,
    hooks::{use_id_or, use_unique_id},
};

// #[expect(clippy::unpredic)]
// Define the properties for our LazyList component
#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct LazyListProps<T: Clone + PartialEq + std::fmt::Display + 'static> {
    id: ReadSignal<Option<String>>,

    focus_scope: Option<ReadSignal<String>>,

    /// The full list of items to display
    /// String is the unique key and T is the element to render
    items: ReadSignal<Arc<[T]>>,
    /// A function that takes an item and returns the RSX for it.
    render_item: Component<RenderItemProps<T>>,
    // render_item: Box<dyn Fn(&T) -> Element<'a> + 'a>,
    /// Max number of items when all are loaded
    #[props(optional)]
    items_count: usize,
    /// Number of items to render above/below the visible area (improves smooth scrolling)
    #[props(default = 5)]
    overscan: usize,
    /// Optional: Classes for the outer container
    #[props(default = String::new())]
    class: String,
    /// Optional: Classes for the inner scroll area
    #[props(default = String::new())]
    item_class: String,
    /// Stable ID prefix for list elements (helps JS interop)
    #[props(default = "virt-item".to_owned())]
    item_id_prefix: String,

    #[props(optional)]
    tabindex: String,

    /// The scroll position is reported to the parent on change and used on first render only when set
    ///
    /// This allows to set the initial scroll position of the list with the last stored scroll position
    #[props(default = Signal::default())]
    scroll_position: Signal<f64>,

    #[props(default = Signal::default())]
    item_height: Signal<f64>,

    #[props(optional)]
    scroll_into_view_index: ReadSignal<Option<usize>>, // Adding these attributes below caused the list to not render anymore
                                                       // #[props(extends = GlobalAttributes)]
                                                       // attributes: Vec<Attribute>,
}

// Define the properties passed to the item renderer function
#[derive(Props, Clone, Debug, Eq, PartialEq)]
pub(crate) struct RenderItemProps<T: Clone + PartialEq + 'static> {
    pub item: T,
    pub index: usize,
    // /// Stable ID for this item element
    // pub id: String,
}

#[component]
pub fn LazyList<T: Clone + PartialEq + std::fmt::Display + 'static>(
    props: LazyListProps<T>,
) -> Element {
    let mut scroll_container = use_signal::<Option<Rc<MountedData>>>(|| None);

    let mut derived_item_height = use_signal(|| 0.0);
    let mut container_height = use_signal(|| 0.0);
    let mut measurement_generation = use_signal(|| 0u32);

    let gen_id = use_unique_id();
    let container_id = use_id_or(gen_id, props.id);

    // restore previous scroll position
    #[expect(unused_results, reason = "There is no meaningful return value")]
    use_effect(move || {
        let item_height: f64 = *derived_item_height.read();

        let scroll_pos = if let Some(index) = props.scroll_into_view_index.read().as_ref()
        // && let Some(pos) = props.items.read().iter().position(|item| item == item_scroll_to)
        {
            item_height * *index as f64
        } else {
            *props.scroll_position.peek()
        };

        #[cfg(feature = "web")]
        if let Some(element) = scroll_container.read().as_ref() {
            element.as_web_event().set_scroll_top(scroll_pos as i32);
        }

        #[cfg(feature = "desktop")]
        #[expect(unused_results, reason = "There is no meaningful return value")]
        spawn(async move {
            #[expect(unused_must_use, reason = "There is no meaningful return value")]
            dioxus::document::eval(&format!(
                r#"document.getElementById("{}"]').scrollTo(0, {});"#,
                container_id.peek(),
                scroll_pos
            ))
            .await;
        });
    });

    // Re-probe container height when items or selection changes.
    // Handles the case where LazyList mounts inside a closed <dialog>: onmounted fires
    // with height=0, and onresize doesn't fire on showModal(), so heights_known stays false.
    #[expect(unused_results, reason = "There is no meaningful return value")]
    use_effect(move || {
        let _ = props.items.read();
        let _ = props.scroll_into_view_index.read();

        if *derived_item_height.peek() == 0.0 {
            let container = scroll_container.read().as_ref().cloned();
            #[expect(
                unused_results,
                reason = "Reconsider if we need to do something with the returned task"
            )]
            spawn(async move {
                if let Some(container) = container {
                    if let Ok(rect) = container.get_client_rect().await {
                        if rect.size.height > 0.0 {
                            container_height.set(rect.size.height);
                            *measurement_generation.write() += 1;
                        }
                    }
                }
            });
        }
    });

    // Calculate derived state: visible items range
    let visible_range = use_memo(move || {
        let derived_h = *derived_item_height.read();
        let container_h = *container_height.read();
        let num_items = props.items.read().len();

        // warn!("num_items: {:?} derived_height: {}", num_items, derived_h);
        // Only perform calculations if we have valid dimensions
        if num_items == 0 || derived_h <= 0.0 || container_h <= 0.0 {
            // If height is unknown, default to rendering only the first item for measurement
            // `usize::from(bool)` Converts false to 0 and true to 1
            return 0..usize::from(num_items > 0);
        }
        let first_visible_index = (*props.scroll_position.read() / derived_h).floor() as usize;
        let visible_item_count = (container_h / derived_h).ceil() as usize;

        let start_index = first_visible_index.saturating_sub(props.overscan);
        let end_index = (first_visible_index + visible_item_count + props.overscan).min(num_items);

        // warn!("visible_range calc: {:?}", start_index..end_index);
        start_index..end_index
    });

    // warn!("visible_range: {:?}", visible_range.read());

    // Calculate total height and offset based on *derived* item height
    let current_item_height = *derived_item_height.read();
    let total_height = if current_item_height > 0.0 {
        props.items.read().len() as f64 * current_item_height
    } else {
        0.0 // Cannot determine total height yet
    };
    let visible_window_offset = if current_item_height > 0.0 {
        visible_range.read().start as f64 * current_item_height
    } else {
        0.0
    };

    // Determine if we are ready to render the virtualized list
    let heights_known = current_item_height > 0.0 && (*container_height.read()) > 0.0;
    // warn!("container_height: {}", container_height.read());
    let render_item = props.render_item;
    let visible_items = props
        .items
        .read()
        .get(visible_range())
        .unwrap_or_default()
        .to_vec();

    // TODO(zoechi): this one is never called, the one for the items below only on focus but not on blur
    let focus_handler = move |add: bool| {
        let focus_provider = try_use_context::<Signal<FocusProvider>>();
        if let Some(scope) = props.focus_scope
            && let Some(mut focus_provider) = focus_provider
        {
            #[expect(
                unused_results,
                reason = "Reconsider if we need to do something with the returned task"
            )]
            spawn(async move {
                if add {
                    warn!("focus list");
                    focus_provider.write().add(scope.read().clone()).await;
                } else {
                    warn!("blur list");
                    focus_provider.write().remove(scope.read().clone()).await;
                }
            });
        }
    };

    let item_focus_handler = move |add: bool, id: String| {
        let focus_provider = try_use_context::<Signal<FocusProvider>>();

        if let Some(mut focus_provider) = focus_provider {
            #[expect(
                unused_results,
                reason = "Reconsider if we need to do something with the returned task"
            )]
            spawn(async move {
                if add {
                    warn!("focus list item");
                    focus_provider.write().add(id).await;
                } else {
                    warn!("blur list item");
                    focus_provider.write().remove(id).await;
                }
            });
        }
    };

    let mut onresize_item_height = props.item_height;
    let mut item_onresize_item_height = props.item_height;
    let mut measure_li_onresize_item_height = props.item_height;

    // Stores the measurement li's mounted element so onresize can re-probe it.
    let mut measure_li_mounted = use_signal::<Option<Rc<MountedData>>>(|| None);

    rsx! {
        div { // Outer scrollable container
            id: container_id,
            class: "bwu-list relative overflow-y-auto {props.class} h-full",
            tabindex: props.tabindex,
            onfocusin: move |_| focus_handler(true),
            onfocusout: move |_| focus_handler(false),

            // height: "100%", // Example: Ensure parent defines height, or set fixed h- like h-96
            onscroll: move |_| {
                #[expect(unused_results, reason = "Reconsider how to better handle this")]
                spawn(async move {
                    #[expect(
                        clippy::expect_used,
                        reason = "It doesn't look like this could actually happen"
                    )]
                    let container: Rc<MountedData> = scroll_container
                        .read()
                        .clone()
                        .expect("The container to always exist when this event arrives");
                    #[expect(
                        clippy::expect_used,
                        reason = "It doesn't look like this could actually happen"
                    )]
                    let scroll_offset = container
                        .get_scroll_offset()
                        .await
                        .expect("Offset to return a valid value when it emits this event");
                    if heights_known {
                        let mut scroll_position = props.scroll_position;
                        scroll_position.set(scroll_offset.y);
                    }
                });
            },

            onresize: move |_cx| async move {
                if let Some(scroll_container) = scroll_container.read().as_ref() && let Ok(rect) = scroll_container.get_client_rect().await {
                    let was_hidden = *container_height.peek() == 0.0;
                    container_height.set(rect.size.height);
                    onresize_item_height.set(0.0);
                    // When container becomes visible (e.g. a dialog opens), the initial
                    // measurement fired with zero height. Reset and force the measurement
                    // li to remount so onmounted fires again with real dimensions.
                    if was_hidden && rect.size.height > 0.0 {
                        derived_item_height.set(0.0);
                        *measurement_generation.write() += 1;
                    }
                }
            },
            onmounted: move |evt| async move {
                scroll_container.set(Some(evt.data()));
                if let Ok(rect) = evt.data().get_client_rect().await {
                    // warn!("onmounted");
                    container_height.set(rect.size.height);
                    derived_item_height.set(0.0);
                    // onresize_item_height.set(0.0);
                }
            },
            // ..props.attributes, // this caused the list to not render anymore

            // Inner container responsible for total height
            // class: "list-row relative {props.item_class}",
            // class: "list  relative",
            ul { style: "height: {total_height}px;", class: "list",
                // --- Conditional Rendering & Measurement ---
                if !heights_known && !props.items.read().is_empty() {
                    // Render ONLY the first item if heights are not known yet,
                    // giving it the specific ID for measurement.
                    // measurement_generation changes when the container becomes visible
                    // after being hidden (e.g. a dialog opens), forcing this li to
                    // remount and re-fire onmounted with real dimensions.
                    li {
                        key: "{props.item_id_prefix}-measure-{measurement_generation}", // Stable key
                        class: props.item_class,
                        tabindex: "0",
                        onmounted: move |evt| async move {
                            let data = evt.data();
                            measure_li_mounted.set(Some(data.clone()));
                            if let Ok(rect) = data.get_client_rect().await {
                                if rect.size.height > 0.0 {
                                    derived_item_height.set(rect.size.height);
                                    item_onresize_item_height.set(rect.size.height);
                                }
                            }
                        },
                        // Item content may load after onmounted (async coroutine); re-probe
                        // when the li grows so derived_item_height gets the real value.
                        onresize: move |_| async move {
                            // Clone out before awaiting so the signal borrow does not span
                            // the await point; concurrent onmounted calls .set() on the same
                            // signal and would panic with AlreadyBorrowed otherwise.
                            let li_opt = { measure_li_mounted.read().as_ref().cloned() };
                            if let Some(li) = li_opt {
                                if let Ok(rect) = li.get_client_rect().await {
                                    if rect.size.height > 0.0 {
                                        derived_item_height.set(rect.size.height);
                                        measure_li_onresize_item_height.set(rect.size.height);
                                    }
                                }
                            }
                        },
                        render_item {
                            item: #[expect(clippy::indexing_slicing, reason = "We check above that it is not empty")]
                            props.items.read()[0].clone(),
                            index: 0,
                            // ID used by JS to measure
                            // id: "{props.item_id_prefix}-0",
                        }
                    }
                } else if heights_known {
                    // Render the virtualized window when heights are known
                    {
                        visible_items
                            .iter()
                            .enumerate()
                            .map(|(relative_index, item)| {
                                let actual_index = visible_range.read().start + relative_index;
                                let item_id = format!("{}-{}", props.item_id_prefix, item.clone());
                                let item_id_onblur = item_id.clone();
                                let item_id_onfocus = item_id;

                                rsx! {
                                    li {
                                        class: "{props.item_class}",
                                        style: "position: absolute; top: { relative_index as f64 * current_item_height + visible_window_offset}px; left: 0; right: 0;",
                                        tabindex: "0",
                                        onfocusin: move |_| item_focus_handler(true, item_id_onfocus.clone()),
                                        onfocusout: move |_| item_focus_handler(false, item_id_onblur.clone()),

                                        render_item {
                                            item: item.clone(),
                                            index: actual_index,
                                            // id, // Pass ID to renderer
                                        }
                                    }
                                }
                            })
                    }
                }
                        // Else: Render nothing if items are empty or heights still unknown
            }
        }
    }
}

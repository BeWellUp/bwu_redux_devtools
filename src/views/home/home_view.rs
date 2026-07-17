use std::collections::HashMap;

use bwu_redux_devtools::redux::{Action, Store};
use dioxus::prelude::*;
use dioxus_free_icons::{
    Icon,
    icons::ld_icons::{LdAppWindow, LdChevronLeft, LdChevronRight},
};

use super::HomeViewFacade;
use crate::{
    components::daisyui::{
        Drawer, DrawerMenu, Kbd, KeyboardKey, MenuItemTrigger, MenuTitle, RouteMenuItem, Tooltip,
        TooltipContent, TooltipPlacement,
    },
    route::Route,
};

#[component]
pub(crate) fn HomeView() -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| HomeViewFacade::new(store.clone()));

    let full_route = use_route::<Route>();
    let nav = use_navigator();

    let app_names = facade.read().get_app_names();

    let tooltip_placement = TooltipPlacement::Right;
    let use_portal = true;

    let access_keys = HashMap::from([
        (0, "a"),
        (1, "b"),
        (2, "c"),
        (3, "g"),
        (4, "h"),
        (5, "i"),
        (6, "j"),
        (7, "l"),
        (8, "m"),
        (9, "n"),
    ]);

    rsx! {
        Drawer {
            trigger_id: "trigger_id",
            label: "Label",
            // page_content_class: "flex items-center p-4 border-b border-base-300",
            page_content: rsx! {
                PageContent {}
                // div { "some text" }
                // Progress { class: "w-56", color: ProgressColor::Info, value: 40 }
                footer { class: "page-footer" }
            },
            DrawerMenu {
                MenuItemTrigger {
                    class: "drawer-left-open-close",
                    r#for: "trigger_id",
                    accesskey: "S",
                    Tooltip {
                        use_portal,
                        tooltip_placement,
                        tooltip_content: rsx! {
                            TooltipContent { class: "nowrap",
                                "Open/close sidebar  "
                                Kbd { "{KeyboardKey::Alt}" }
                                " + "
                                Kbd { "S" }
                            }
                        },
                        Icon { class: "icon drawer-close", icon: LdChevronLeft }
                        Icon { class: "icon drawer-open", icon: LdChevronRight }
                    }
                }

                MenuTitle { "Apps" }

                for (index , (app_id , app_name)) in app_names().into_iter().enumerate() {
                    RouteMenuItem {
                        key: "{app_id}",
                        accesskey: if index <= 9 { access_keys[&index] },
                        is_active: full_route
                            == Route::AppStateView {
                                app_id: app_id.to_string(),
                            },
                        onclick: move |_| {
                            facade.read().dispatch(Action::SelectedAppChange {
                                app_id: app_id.clone(),
                            });
                            let _ = nav.push(Route::AppStateView {
                                app_id: app_id.to_string(),
                            });
                        },
                        div {
                            Tooltip {
                                use_portal,
                                tooltip_content: rsx! {
                                    TooltipContent { class: "nowrap",
                                        "{app_name}"
                                        if index <= 9 {
                                            Kbd { "{KeyboardKey::Alt}" }
                                            " + "
                                            Kbd { "{access_keys[&index]}" }
                                        }
                                    }
                                },
                                tooltip_placement,
                                Icon { class: "icon", icon: LdAppWindow }
                            }

                            span { class: "menu-text", "{app_name}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PageContent() -> Element {
    rsx! {
        div { class: "page-content", Outlet::<Route> {} }
    }
}

#[component]
pub(crate) fn NotFoundView(segments: Vec<String>) -> Element {
    let nav = navigator();
    let _ = use_effect(move || {
        let _ = nav.replace(Route::NoAppSelected);
    });
    rsx! {}
}

#[component]
pub(crate) fn NoAppSelected() -> Element {
    rsx! {
        div { class: "no-app-selected",
            "Waiting for a Redux store connection…"
        }
    }
}

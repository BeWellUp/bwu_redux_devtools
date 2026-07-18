use bwu_redux_devtools::redux::{Action, Store, app_id::AppId};
use dioxus::prelude::*;
use dioxus_free_icons::{Icon, icons::ld_icons::LdX};

use super::AppSettingsFacade;

#[derive(Props, Clone, PartialEq)]
pub(crate) struct AppSettingsPageProps {
    pub(crate) app_id: AppId,
    pub(crate) app_name: String,
}

/// Settings tab content: a "Global" section (currently just the theme
/// picker) and an "{`app_name`} settings" section (history limit,
/// drop-history-on-reconnect, paused actions) — replaces the P3 dialog.
#[component]
pub(crate) fn AppSettingsPage(props: AppSettingsPageProps) -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| AppSettingsFacade::new(store.clone()));

    let history_limit = facade.read().get_history_limit();
    let drop_history_on_reconnect = facade.read().get_drop_history_on_reconnect();
    let paused_actions = facade.read().get_paused_actions();

    let app_id = props.app_id;
    let app_name = props.app_name;

    rsx! {
        div { class: "app-settings-page",
            section { class: "app-settings-section",
                h3 { class: "settings-subtitle", "Global" }
                ThemeSelect {}
            }

            section { class: "app-settings-section",
                h3 { class: "settings-subtitle", "{app_name} settings" }

                fieldset { class: "fieldset",
                    legend { class: "fieldset-legend", "History" }

                    label { class: "fieldset-label",
                        "Keep the last"
                        input {
                            r#type: "number",
                            class: "input input-sm w-20",
                            min: "1",
                            value: "{history_limit()}",
                            onchange: move |evt| {
                                if let Ok(limit) = evt.value().parse::<usize>()
                                    && limit >= 1
                                {
                                    facade.read().dispatch(Action::HistoryLimitChange { app_id, limit });
                                }
                            },
                        }
                        "entries"
                    }

                    label { class: "fieldset-label",
                        input {
                            r#type: "checkbox",
                            class: "checkbox checkbox-sm",
                            checked: drop_history_on_reconnect(),
                            onchange: move |evt| {
                                facade
                                    .read()
                                    .dispatch(Action::DropHistoryOnReconnectChange {
                                        app_id,
                                        enabled: evt.checked(),
                                    });
                            },
                        }
                        "Drop history when the app reconnects"
                    }
                }

                fieldset { class: "fieldset",
                    legend { class: "fieldset-legend", "Paused actions" }
                    if paused_actions().is_empty() {
                        p { class: "text-sm opacity-60",
                            "No actions paused. Hover an action in the history list and click the mute icon to pause it."
                        }
                    } else {
                        ul { class: "paused-actions-list",
                            for prefix in paused_actions() {
                                li { key: "{prefix}", class: "paused-action-row",
                                    span { "{prefix}" }
                                    button {
                                        class: "btn btn-ghost btn-xs btn-circle",
                                        "aria-label": "Un-pause {prefix}",
                                        onclick: move |_| {
                                            let mut next = paused_actions();
                                            let _ = next.remove(&prefix);
                                            facade
                                                .read()
                                                .dispatch(Action::PauseActionsChange {
                                                    app_id,
                                                    paused_prefixes: next,
                                                });
                                        },
                                        Icon { icon: LdX }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// DaisyUI theme picker; the choice is persisted by `StorageMiddleware`.
#[component]
fn ThemeSelect() -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| AppSettingsFacade::new(store.clone()));

    let themes = facade.read().get_themes();
    let selected_theme = facade.read().get_selected_theme();

    rsx! {
        label { class: "theme-select",
            "Theme"
            select {
                class: "select select-sm",
                onchange: move |evt| {
                    facade.read().dispatch(Action::ThemeChange { theme: evt.value() });
                },
                for name in themes() {
                    option { value: "{name}", selected: name == selected_theme(), "{name}" }
                }
            }
        }
    }
}

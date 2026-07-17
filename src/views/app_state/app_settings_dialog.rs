use bwu_redux_devtools::redux::{Action, Store, app_id::AppId};
use dioxus::prelude::*;

use super::AppSettingsFacade;
use crate::components::dialog::{Dialog, DialogDescription, DialogTitle};

#[derive(Props, Clone, PartialEq)]
pub(crate) struct AppSettingsDialogProps {
    pub(crate) app_id: AppId,
    pub(crate) open: Signal<bool>,
}

/// Per-app settings (history limit, drop-history-on-reconnect); opened from
/// a gear button in `AppStateView`.
#[component]
pub(crate) fn AppSettingsDialog(props: AppSettingsDialogProps) -> Element {
    let store = use_context::<Store>();
    let facade = use_signal(|| AppSettingsFacade::new(store.clone()));

    let history_limit = facade.read().get_history_limit();
    let drop_history_on_reconnect = facade.read().get_drop_history_on_reconnect();

    let app_id = props.app_id;
    let mut open = props.open;

    rsx! {
        Dialog {
            open: use_memo(move || Some(open())),
            on_open_change: move |value: bool| open.set(value),
            DialogTitle { "App settings" }
            DialogDescription { "Applies to this app; saved for future connections." }

            button {
                class: "dx-dialog-close",
                "aria-label": "Close",
                onclick: move |_| open.set(false),
                "\u{2715}"
            }

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
        }
    }
}

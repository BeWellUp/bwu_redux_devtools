use std::collections::BTreeSet;

use rpds::VectorSync;

use super::{GlobalCounter, StateChangeMessage, StateViewer, app_id::AppId};

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
    DropHistoryOnReconnectChange {
        app_id: AppId,
        enabled: bool,
    },
    Error(Error),
    Exit,
    HistoryLimitChange {
        app_id: AppId,
        limit: usize,
    },
    /// Replaces the whole paused-actions set for an app. Dispatched by the
    /// settings dialog on every checkbox toggle, and by `StorageMiddleware`
    /// to re-send a persisted set to the server when the app first connects
    /// in a GUI session.
    PauseActionsChange {
        app_id: AppId,
        paused_prefixes: BTreeSet<String>,
    },
    ReduxStateChange(ReduxStateChange),
    SelectedAppChange {
        app_id: AppId,
    },
    SelectedStateChange {
        counter: GlobalCounter,
    },
    StateUpdate {
        app_id: AppId,
        app_name: String,
        content: VectorSync<StateChangeMessage>,
    },
    StateUpdateFailure {
        error: String,
    },
    StateViewerChange(StateViewer),
    ThemeChange {
        theme: String,
    },
}

#[cfg(feature = "redux_devtools")]
impl TryFrom<bwu_redux::devtools_rpc::StateChangeRequest> for Action {
    type Error = String;

    // `Self::Error` would be ambiguous with the `Action::Error` variant.
    fn try_from(req: bwu_redux::devtools_rpc::StateChangeRequest) -> Result<Self, String> {
        let app_id = req
            .app_id
            .ok_or_else(|| String::from("missing app_id in StateChangeRequest"))?;
        let app_id = uuid::Uuid::parse_str(app_id.value.as_str())
            .map_err(|err| format!("invalid app_id UUID in StateChangeRequest: {err}"))?;

        Ok(Self::StateUpdate {
            app_id: app_id.into(),
            app_name: req.app_name,
            content: req
                .changes
                .into_iter()
                .map(Into::into)
                .collect::<VectorSync<_>>(),
        })
    }
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReduxStateChange {
    StoreInit,
    AppInit,
    Close,
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    ConfigReadFailure(String),
    ConfigWriteFailure(String),
    LocalStorageReadFailure(String),
    LocalStorageWriteFailure(String),
    ThemeDoesNotExist(String),
}

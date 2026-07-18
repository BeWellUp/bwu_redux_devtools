#[cfg(target_family = "wasm")]
use std::collections::BTreeSet;
use std::sync::Arc;
#[cfg(not(target_family = "wasm"))]
use std::{
    fs::File,
    io::{Read as _, Write as _},
    path::PathBuf,
};

use bwu_redux::{Middleware, MiddlewareRef, StoreWrapper};
#[cfg(not(target_family = "wasm"))]
use dioxus::logger::tracing::debug;
#[cfg(target_family = "wasm")]
use gloo_storage::{LocalStorage, Storage};

use super::{Action, Error, ReduxStateChange, State, app_id::AppId};

#[derive(Copy, Clone, Debug, Default)]
#[non_exhaustive]
pub(crate) struct StorageMiddleware;

#[cfg(not(target_family = "wasm"))]
use config::Config;
#[cfg(not(target_family = "wasm"))]
use directories::ProjectDirs;
#[cfg(not(target_family = "wasm"))]
use toml_edit::{Array, DocumentMut, Item, Table, Value};

#[cfg(target_family = "wasm")]
const THEME_NAME_STOREAGE_KEY: &str = "bwu_redux_devtools::theme_name";

#[cfg(not(target_family = "wasm"))]
const THEME_NAME_CONFIG_KEY: &str = "theme-name";

const DEFAULT_THEME_NAME: &str = "default";

const HISTORY_LIMIT_FIELD: &str = "history-limit";
const DROP_HISTORY_ON_RECONNECT_FIELD: &str = "drop-history-on-reconnect";
const PAUSED_ACTIONS_FIELD: &str = "paused-actions";

#[cfg(target_family = "wasm")]
fn app_setting_key(app_id: AppId, field: &str) -> String {
    format!("bwu_redux_devtools::app::{app_id}::{field}")
}

#[cfg(not(target_family = "wasm"))]
fn app_setting_key(app_id: AppId, field: &str) -> String {
    format!("apps.{app_id}.{field}")
}

impl Middleware<State, Action> for StorageMiddleware {
    fn apply(
        &self,
        store: Arc<StoreWrapper<State, Action>>,
        action: Action,
        next: Arc<MiddlewareRef<State, Action>>,
    ) {
        let tx = store.get_dispatch_sender();
        match action.clone() {
            Action::ReduxStateChange(ReduxStateChange::StoreInit) => {
                let result: Action;
                #[cfg(target_family = "wasm")]
                {
                    result = match LocalStorage::get(THEME_NAME_STOREAGE_KEY) {
                        Ok(theme) => Action::ThemeChange { theme },

                        Err(_err) => Action::ThemeChange {
                            theme: DEFAULT_THEME_NAME.into(),
                        },
                    };
                };
                #[cfg(not(target_family = "wasm"))]
                {
                    result = match get_config(&action, true) {
                        Ok(settings) => match settings.get_string(THEME_NAME_CONFIG_KEY) {
                            Ok(theme) => {
                                debug!("==== found theme {theme:?} =====");
                                Action::ThemeChange { theme }
                            }
                            // Err(err) => Action::Error(Error::ConfigReadFailure(format!(
                            //     "{:?} {:?}",
                            //     action, err,
                            // ))),
                            Err(err) => {
                                debug!("==== get settings string error {err:?} =====");
                                Action::ThemeChange {
                                    theme: DEFAULT_THEME_NAME.into(),
                                }
                            }
                        },
                        Err(err) => {
                            debug!("==== get settings error {err:?} =====");
                            err
                        }
                    };
                };

                let _ = tx.send(result);
            }

            Action::ThemeChange { theme } => {
                let result: Option<Action>;
                #[cfg(target_family = "wasm")]
                {
                    result = match LocalStorage::set(THEME_NAME_STOREAGE_KEY, theme) {
                        Err(err) => Some(Action::Error(Error::LocalStorageWriteFailure(format!(
                            "{:?} {:?}",
                            action, err,
                        )))),
                        _ => None,
                    };
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    result =
                        write_config_value(&[THEME_NAME_CONFIG_KEY], Item::from(theme), &action)
                            .err();
                };
                let _ = result.map(move |result| {
                    let _ = tx.send(result);
                });
            }

            // The first `StateUpdate` for an app in this GUI session is the
            // moment to load its persisted per-app settings; once the app
            // exists in the store, further `StateUpdate`s for it are no
            // longer "new" and skip this (so a live in-session change made
            // via the settings dialog is never clobbered by a stale reload).
            Action::StateUpdate { app_id, .. } => {
                let is_new_app =
                    store.select(move |state: &State| state.app_states.get(&app_id).is_none());
                if is_new_app {
                    #[cfg(target_family = "wasm")]
                    {
                        if let Ok(limit) =
                            LocalStorage::get::<usize>(app_setting_key(app_id, HISTORY_LIMIT_FIELD))
                        {
                            let _ = tx.send(Action::HistoryLimitChange { app_id, limit });
                        }
                        if let Ok(enabled) = LocalStorage::get::<bool>(app_setting_key(
                            app_id,
                            DROP_HISTORY_ON_RECONNECT_FIELD,
                        )) {
                            let _ =
                                tx.send(Action::DropHistoryOnReconnectChange { app_id, enabled });
                        }
                        // Re-send any persisted pause set to the server: its
                        // in-memory pause state doesn't survive a server
                        // restart, so a GUI's stated preference has to be
                        // reasserted whenever the app (re)connects.
                        if let Ok(paused_prefixes) = LocalStorage::get::<BTreeSet<String>>(
                            app_setting_key(app_id, PAUSED_ACTIONS_FIELD),
                        ) && !paused_prefixes.is_empty()
                        {
                            let _ = tx.send(Action::PauseActionsChange {
                                app_id,
                                paused_prefixes,
                            });
                        }
                    }
                    #[cfg(not(target_family = "wasm"))]
                    {
                        if let Ok(settings) = get_config(&action, true) {
                            if let Ok(limit) =
                                settings.get_int(&app_setting_key(app_id, HISTORY_LIMIT_FIELD))
                            {
                                let _ = tx.send(Action::HistoryLimitChange {
                                    app_id,
                                    limit: usize::try_from(limit).unwrap_or_default(),
                                });
                            }
                            if let Ok(enabled) = settings
                                .get_bool(&app_setting_key(app_id, DROP_HISTORY_ON_RECONNECT_FIELD))
                            {
                                let _ = tx
                                    .send(Action::DropHistoryOnReconnectChange { app_id, enabled });
                            }
                            if let Ok(paused_prefixes) = settings
                                .get::<Vec<String>>(&app_setting_key(app_id, PAUSED_ACTIONS_FIELD))
                                && !paused_prefixes.is_empty()
                            {
                                let _ = tx.send(Action::PauseActionsChange {
                                    app_id,
                                    paused_prefixes: paused_prefixes.into_iter().collect(),
                                });
                            }
                        }
                    }
                }
            }

            Action::HistoryLimitChange { app_id, limit } => {
                #[cfg(target_family = "wasm")]
                {
                    let _ = LocalStorage::set(app_setting_key(app_id, HISTORY_LIMIT_FIELD), limit);
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    let _ = write_config_value(
                        &["apps", &app_id.to_string(), HISTORY_LIMIT_FIELD],
                        Item::from(i64::try_from(limit).unwrap_or(i64::MAX)),
                        &action,
                    );
                }
            }

            Action::DropHistoryOnReconnectChange { app_id, enabled } => {
                #[cfg(target_family = "wasm")]
                {
                    let _ = LocalStorage::set(
                        app_setting_key(app_id, DROP_HISTORY_ON_RECONNECT_FIELD),
                        enabled,
                    );
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    let _ = write_config_value(
                        &["apps", &app_id.to_string(), DROP_HISTORY_ON_RECONNECT_FIELD],
                        Item::from(enabled),
                        &action,
                    );
                }
            }

            Action::PauseActionsChange {
                app_id,
                paused_prefixes,
            } => {
                #[cfg(target_family = "wasm")]
                {
                    let _ = LocalStorage::set(
                        app_setting_key(app_id, PAUSED_ACTIONS_FIELD),
                        &paused_prefixes,
                    );
                }
                #[cfg(not(target_family = "wasm"))]
                {
                    let mut array = Array::new();
                    for prefix in &paused_prefixes {
                        array.push(prefix.as_str());
                    }
                    let _ = write_config_value(
                        &["apps", &app_id.to_string(), PAUSED_ACTIONS_FIELD],
                        Item::Value(Value::Array(array)),
                        &action,
                    );
                }
            }

            _ => {}
        }

        next.apply(store, action);
    }
}

#[cfg(not(target_family = "wasm"))]
fn write_config_value(toml_path: &[&str], value: Item, action: &Action) -> Result<(), Action> {
    match get_config_file_path(action) {
        Ok(mut config_file_path) => {
            // let config_file = File::from(config_file_path);
            let _ = config_file_path.set_extension("toml");
            let doc = match config_file_path.try_exists() {
                Ok(exists) => {
                    if exists {
                        match File::open(&config_file_path) {
                            Ok(mut file) => {
                                let mut buf = String::new();
                                match file.read_to_string(&mut buf) {
                                    Ok(_num_bytes) => match buf.parse::<DocumentMut>() {
                                        Ok(doc) => Ok(doc),
                                        Err(err) => {
                                            Err(Action::Error(Error::ConfigReadFailure(format!(
                                                "{action:?} parsing the config file's {config_file_path:?} content failed {err:?}"
                                            ))))
                                        }
                                    },
                                    Err(err) => {
                                        Err(Action::Error(Error::ConfigReadFailure(format!(
                                            "{action:?} reading the config file's {config_file_path:?} content failed {err:?}"
                                        ))))
                                    }
                                }
                            }
                            Err(err) => Err(Action::Error(Error::ConfigReadFailure(format!(
                                "{action:?} the config file {config_file_path:?} seems to exist but opening it failed {err:?}"
                            )))),
                        }
                    } else {
                        match config_file_path.parent().ok_or(Action::Error(Error::ConfigReadFailure(format!(
                                "{action:?} there is an issue with the config file path {config_file_path:?}"
                        )))).and_then(|dir| std::fs::create_dir_all(dir).map_err(|err|
                            Action::Error(Error::ConfigReadFailure(format!(
                                "{action:?} the config file's parent directory couln't be created {config_file_path:?} {err:?}"
                            ))))
                        )

                         {
                            Ok(()) => Ok(DocumentMut::new()),
                            Err(err) => Err(err),
                        }
                    }
                }
                Err(err) => Err(Action::Error(Error::ConfigReadFailure(format!(
                    "{action:?} there is an issue with the config file {config_file_path:?} {err:?}"
                )))),
            };

            match doc {
                Ok(mut doc) => match set_nested_value(&mut doc, toml_path, value) {
                    Ok(()) => match File::create(&config_file_path) {
                        Ok(mut file) => match file.write_all(doc.to_string().as_bytes()) {
                            Ok(()) => Ok(()),
                            Err(err) => Err(Action::Error(Error::ConfigWriteFailure(format!(
                                "{action:?} writing the config file's {config_file_path:?} failed {err:?}"
                            )))),
                        },
                        Err(err) => Err(Action::Error(Error::ConfigWriteFailure(format!(
                            "{action:?} creating the config file's {config_file_path:?} failed {err:?}"
                        )))),
                    },
                    Err(err) => Err(Action::Error(Error::ConfigWriteFailure(format!(
                        "{action:?} updating the config file's {config_file_path:?} content failed {err:?}"
                    )))),
                },
                Err(err) => Err(err),
            }
        }
        Err(err) => Err(err),
    }
}

#[cfg(not(target_family = "wasm"))]
fn get_config(action: &Action, include_environment: bool) -> Result<Config, Action> {
    match get_config_file_path(action) {
        Ok(config_file) => match config_file.to_str() {
            Some(file_path) => {
                debug!("===== get_config config_file {config_file:?} ====");
                let mut builder = Config::builder().add_source(config::File::with_name(file_path));
                if include_environment {
                    builder = builder.add_source(config::Environment::with_prefix("BWU_REDUX"));
                }

                match builder.build() {
                    Ok(settings) => Ok(settings),
                    Err(err) => Err(Action::Error(Error::ConfigReadFailure(format!(
                        "{action:?} {err:?}",
                    )))),
                }
            }
            None => Err(Action::Error(Error::ConfigReadFailure(format!(
                "{action:?} The path is not a valid path: {config_file:?}"
            )))),
        },
        Err(err) => Err(err),
    }
}

#[cfg(not(target_family = "wasm"))]
fn get_config_file_path(action: &Action) -> Result<PathBuf, Action> {
    if let Some(project_dir) = ProjectDirs::from("net", "zoechbauer", "bwu_redux_devtools") {
        let config_dir = project_dir.config_dir();
        Ok(config_dir.join("settings"))
    } else {
        Err(Action::Error(Error::ConfigReadFailure(format!(
            "{action:?} Faild to find the home directory",
        ))))
    }
}

/// Sets a value at a specific key path within a TOML document.
/// Creates intermediate tables if they don't exist.
///
/// # Arguments
///
/// * `doc` - A mutable reference to the TOML document.
/// * `path` - A slice of strings representing the keys to traverse (e.g., `&["database", "connection", "pool_size"]`).
/// * `value_to_set` - The TOML `Item` to insert at the final path location.
///
/// # Returns
///
/// * `Ok(())` if the value was set successfully.
/// * `Err(SetValueError)` if the path was invalid or an intermediate item was not a table.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn set_nested_value(
    doc: &mut DocumentMut,
    path: &[&str],
    value_to_set: Item,
) -> Result<(), SetValueError> {
    if path.is_empty() {
        return Err(SetValueError::EmptyPath);
    }

    // The key for the final value
    let final_key = path.last().unwrap(); // Safe due to the empty check above
    // The path segments leading to the table containing the final key
    let table_path = &path[..path.len() - 1];

    // Start at the root table of the document
    let mut current_table = doc.as_table_mut();

    // Traverse the path, creating tables as needed
    for segment in table_path {
        // Get the next item, or insert a new empty Table if it doesn't exist
        let next_item = current_table
            .entry(segment)
            .or_insert_with(|| Item::Table(Table::new()));

        // Check if the item (either existing or newly created) is actually a table
        if !next_item.is_table() {
            // It exists but is something else (e.g., an integer, string) - error!
            return Err(SetValueError::NotATable(segment.to_string()));
        }

        // Move down to the next level table (safe due to the is_table check)
        current_table = next_item.as_table_mut().unwrap();
    }

    // We are now in the correct table; insert or replace the final key with the value
    current_table[final_key] = value_to_set; // Using IndexMut here directly inserts/replaces

    Ok(())
}

// Define a custom error type for clarity
#[derive(Debug, thiserror::Error)]
#[cfg(not(target_family = "wasm"))]
pub(crate) enum SetValueError {
    #[error("Invalid path: Path cannot be empty")]
    EmptyPath,
    #[error("Expected a table at segment '{0}', but found a different item type")]
    NotATable(String),
    #[error("Failed to parse TOML: {0}")]
    TomlParseError(#[from] toml_edit::TomlError),
}

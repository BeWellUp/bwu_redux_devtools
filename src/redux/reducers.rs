use std::usize;

use rpds::QueueSync;

use super::{AppState, Error, StateChange};
use crate::redux::{Action, State};

const MAX_HISTORY_ENTRIES: usize = 200;

pub(crate) fn reducer(state: State, action: &Action) -> State {
    let state = match *action {
        Action::Error(ref error) => State {
            errors: state.errors.push_back(error.clone()),
            ..state
        },
        Action::SelectedAppChange { app_id } => State {
            selected_app_id: Some(app_id),
            ..state
        },

        // If `selected_state_id` is set, this on is treated as selected, otherwise the last entry.
        Action::SelectedStateChange { counter } => {
            if let Some(app_id) = state.selected_app_id
                && let Some(app_state) = state.app_states.get(&app_id)
            {
                let selected_state_id = if app_state
                    .selected_state_id
                    .is_some_and(|sel_id| sel_id == counter)
                {
                    None
                } else {
                    Some(counter)
                };

                let app_state = AppState {
                    selected_state_id,
                    ..app_state.clone()
                };

                State {
                    app_states: state.app_states.insert(app_id, app_state),
                    ..state
                }
            } else {
                state
            }
        }

        // If no `app_state` for `app_id` exists, create one and add `content` to its `history`.
        Action::StateUpdate {
            app_id,
            ref app_name,
            ref content,
        } => {
            let mut global_state_counter = state.global_state_counter;
            let app_state = if let Some(app_state) = state.app_states.get(&app_id) {
                let mut history = app_state.history.clone();
                for e in content.iter().rev() {
                    history =
                        history.enqueue(StateChange::from_message(e.clone(), global_state_counter));
                    global_state_counter = global_state_counter.inc();
                }

                while history.len() > MAX_HISTORY_ENTRIES {
                    history = history.dequeue().unwrap_or_default();
                }

                AppState {
                    app_name: app_name.clone(),
                    history,
                    ..app_state.clone()
                }
            } else {
                let history = QueueSync::<super::StateChange>::from_iter(
                    content.into_iter().rev().map(|v| {
                        let result = StateChange::from_message(v.clone(), global_state_counter);
                        global_state_counter = global_state_counter.inc();
                        result
                    }),
                );

                AppState {
                    app_id,
                    app_name: app_name.clone(),
                    history,
                    ..AppState::default()
                }
            };

            State {
                selected_app_id: state.selected_app_id.or(app_id.into()),
                app_states: state.app_states.insert(app_id, app_state),
                global_state_counter,
                ..state
            }
        }

        Action::StateViewerChange(selected_state_viewer) => {
            if let Some(app_id) = state.selected_app_id
                && let Some(app_state) = state.app_states.get(&app_id)
            {
                let app_state = AppState {
                    selected_state_viewer,
                    ..app_state.clone()
                };

                State {
                    app_states: state.app_states.insert(app_id, app_state),
                    ..state
                }
            } else {
                state
            }
        }
        Action::ThemeChange { ref theme } => {
            if state.themes.iter().any(|v| v == theme) {
                State {
                    selected_theme: theme.clone(),
                    ..state
                }
            } else {
                State {
                    errors: state
                        .errors
                        .push_back(Error::ThemeDoesNotExist(theme.clone())),
                    ..state
                }
            }
        }
        // Action::Exit => state,
        _ => state,
    };

    // debug!("{state:?}");
    state
}

use rpds::QueueSync;

use super::{AppState, Error, StateChange};
use crate::redux::{Action, State};

pub(crate) fn reducer(state: State, action: &Action) -> State {
    let state = match *action {
        // These can arrive before the app's first `StateUpdate` (settings
        // are loaded speculatively as soon as an app is seen, see
        // `StorageMiddleware`), so create a default `AppState` stub rather
        // than no-op — the app's first `StateUpdate` preserves it via
        // `..app_state.clone()`, regardless of arrival order.
        Action::DropHistoryOnReconnectChange { app_id, enabled } => {
            let app_state = state.app_states.get(&app_id).map_or_else(
                || AppState {
                    app_id,
                    ..AppState::default()
                },
                Clone::clone,
            );
            let app_state = AppState {
                drop_history_on_reconnect: enabled,
                ..app_state
            };

            State {
                app_states: state.app_states.insert(app_id, app_state),
                ..state
            }
        }
        Action::Error(ref error) => State {
            errors: state.errors.push_back(error.clone()),
            ..state
        },
        Action::HistoryLimitChange { app_id, limit } => {
            let app_state = state.app_states.get(&app_id).map_or_else(
                || AppState {
                    app_id,
                    ..AppState::default()
                },
                Clone::clone,
            );
            let app_state = AppState {
                history_limit: limit,
                ..app_state
            };

            State {
                app_states: state.app_states.insert(app_id, app_state),
                ..state
            }
        }
        Action::PauseActionsChange {
            app_id,
            ref paused_prefixes,
        } => {
            let app_state = state.app_states.get(&app_id).map_or_else(
                || AppState {
                    app_id,
                    ..AppState::default()
                },
                Clone::clone,
            );
            let app_state = AppState {
                paused_actions: paused_prefixes.clone(),
                ..app_state
            };

            State {
                app_states: state.app_states.insert(app_id, app_state),
                ..state
            }
        }
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
                // A lower session counter than the last-seen one means the
                // monitored app restarted (same heuristic `devtools_watch`
                // uses for replay dedup).
                let is_restart = content
                    .iter()
                    .last()
                    .zip(app_state.history.iter().last())
                    .is_some_and(|(first_new, last_known)| {
                        first_new.session_counter < last_known.session_counter
                    });

                let mut history = if is_restart && app_state.drop_history_on_reconnect {
                    QueueSync::default()
                } else {
                    app_state.history.clone()
                };

                for e in content.iter().rev() {
                    history =
                        history.enqueue(StateChange::from_message(e.clone(), global_state_counter));
                    global_state_counter = global_state_counter.inc();
                }

                while history.len() > app_state.history_limit {
                    history = history.dequeue().unwrap_or_default();
                }

                AppState {
                    app_name: app_name.clone(),
                    history,
                    ..app_state.clone()
                }
            } else {
                let history_limit = super::DEFAULT_HISTORY_LIMIT;
                let mut history = QueueSync::<super::StateChange>::from_iter(
                    content.into_iter().rev().map(|v| {
                        let result = StateChange::from_message(v.clone(), global_state_counter);
                        global_state_counter = global_state_counter.inc();
                        result
                    }),
                );

                while history.len() > history_limit {
                    history = history.dequeue().unwrap_or_default();
                }

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

#[cfg(test)]
mod tests {
    use bwu_redux::ActionFilter as _;
    use pretty_assertions::assert_eq;
    use rpds::VectorSync;

    use super::*;
    use crate::redux::{DevToolsActionFilter, GlobalCounter, StateChangeMessage, app_id::AppId};

    fn messages(range: std::ops::Range<u32>) -> VectorSync<StateChangeMessage> {
        range
            .map(|n| StateChangeMessage {
                session_counter: n.into(),
                action: format!("\"action-{n}\""),
                state: format!("{n}"),
            })
            .collect()
    }

    /// Like `messages`, but newest session-counter first — the order a real
    /// client batch arrives in (see `bwu_redux`'s `DevtoolsSender::get_changes`).
    fn messages_desc(range: std::ops::Range<u32>) -> VectorSync<StateChangeMessage> {
        range
            .rev()
            .map(|n| StateChangeMessage {
                session_counter: n.into(),
                action: format!("\"action-{n}\""),
                state: format!("{n}"),
            })
            .collect()
    }

    #[test]
    fn theme_change_selects_known_theme() {
        let state = reducer(
            State::default(),
            &Action::ThemeChange {
                theme: String::from("dark"),
            },
        );
        assert_eq!(state.selected_theme, "dark");
        assert!(state.errors.is_empty());
    }

    #[test]
    fn theme_change_rejects_unknown_theme() {
        let state = reducer(
            State::default(),
            &Action::ThemeChange {
                theme: String::from("no-such-theme"),
            },
        );
        assert_eq!(state.selected_theme, "default");
        assert_eq!(
            state.errors.iter().next(),
            Some(&Error::ThemeDoesNotExist(String::from("no-such-theme")))
        );
    }

    #[test]
    fn state_update_caps_history_at_200() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages(0..150),
            },
        );
        let state = reducer(
            state,
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages(150..250),
            },
        );
        let app_state = state.app_states.get(&app_id).expect("app state");
        assert_eq!(app_state.history.len(), 200);
    }

    #[test]
    fn state_update_selects_first_connected_app() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages(0..1),
            },
        );
        assert_eq!(state.selected_app_id, Some(app_id));
    }

    #[test]
    fn selected_state_change_toggles_selection() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages(0..3),
            },
        );
        let counter = GlobalCounter::from(1_usize);
        let state = reducer(state, &Action::SelectedStateChange { counter });
        assert_eq!(
            state
                .app_states
                .get(&app_id)
                .expect("app")
                .selected_state_id,
            Some(counter)
        );
        let state = reducer(state, &Action::SelectedStateChange { counter });
        assert_eq!(
            state
                .app_states
                .get(&app_id)
                .expect("app")
                .selected_state_id,
            None
        );
    }

    #[test]
    fn state_update_caps_history_on_first_batch_for_new_app() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(0..250),
            },
        );
        let app_state = state.app_states.get(&app_id).expect("app state");
        assert_eq!(app_state.history.len(), 200);
    }

    #[test]
    fn history_limit_change_applies_to_later_batches() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(0..10),
            },
        );
        let state = reducer(state, &Action::HistoryLimitChange { app_id, limit: 5 });
        let state = reducer(
            state,
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(10..20),
            },
        );
        let app_state = state.app_states.get(&app_id).expect("app state");
        assert_eq!(app_state.history.len(), 5);
    }

    #[test]
    fn reconnect_without_drop_keeps_prior_history() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(0..3),
            },
        );
        // Simulate an app restart: the new batch's session counters start
        // over from 0.
        let state = reducer(
            state,
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(0..2),
            },
        );
        let app_state = state.app_states.get(&app_id).expect("app state");
        assert_eq!(app_state.history.len(), 5);
    }

    #[test]
    fn reconnect_with_drop_enabled_clears_prior_history() {
        let app_id = AppId::new();
        let state = reducer(
            State::default(),
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(0..3),
            },
        );
        let state = reducer(
            state,
            &Action::DropHistoryOnReconnectChange {
                app_id,
                enabled: true,
            },
        );
        let state = reducer(
            state,
            &Action::StateUpdate {
                app_id,
                app_name: String::from("app"),
                content: messages_desc(0..2),
            },
        );
        let app_state = state.app_states.get(&app_id).expect("app state");
        assert_eq!(app_state.history.len(), 2);
    }

    #[test]
    fn devtools_filter_excludes_state_updates() {
        let filter = DevToolsActionFilter;
        assert!(!filter.filter(&Action::StateUpdate {
            app_id: AppId::new(),
            app_name: String::from("app"),
            content: VectorSync::default(),
        }));
        assert!(filter.filter(&Action::ThemeChange {
            theme: String::from("dark"),
        }));
    }
}

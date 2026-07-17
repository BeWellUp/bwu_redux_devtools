use std::{borrow::Cow, collections::BTreeSet, sync::Arc};

use bwu_redux::SelectorFn;
use ron::ser::to_string_pretty;
use rpds::QueueSync;

use super::{ChangesStream, GlobalCounter, State, StateChange, StateViewer, Store, app_id::AppId};

pub fn select_action_for_counter(counter: GlobalCounter) -> impl SelectorFn<State, Option<String>> {
    move |state: &State| {
        select_selected_history(state)
            .iter()
            .find_map(|item| (item.counter == counter).then(|| item.action.clone()))
    }
}

pub fn select_change_for_counter(
    counter: GlobalCounter,
) -> impl SelectorFn<State, Option<StateChange>> {
    move |state: &State| {
        select_selected_history(state)
            .iter()
            .find_map(|item| (item.counter == counter).then(|| item.to_owned()))
    }
}

pub fn select_selected_change(state: &State) -> Option<StateChange> {
    let app_state = state
        .selected_app_id
        .and_then(|app_id| state.app_states.get(&app_id))?;
    if let Some(selected_state_id) = app_state.selected_state_id {
        app_state
            .history
            .iter()
            .find_map(|item| (item.counter == selected_state_id).then(|| item.to_owned()))
    } else {
        app_state
            .history
            .iter()
            .last()
            .map(std::borrow::ToOwned::to_owned)
    }
}

pub fn select_selected_app_name(state: &State) -> Option<String> {
    state
        .selected_app_id
        .and_then(|app_id| state.app_states.get(&app_id))
        .map(|app_state| app_state.app_name.clone())
}

pub fn select_selected_app_id(state: &State) -> Option<AppId> {
    state.selected_app_id
}

pub fn select_app_names(state: &State) -> Vec<(AppId, String)> {
    state
        .app_states
        .iter()
        .map(|(&app_id, app_state)| (app_id, app_state.app_name.clone()))
        .collect()
}

pub fn select_selected_history(state: &State) -> QueueSync<StateChange> {
    state
        .selected_app_id
        .and_then(|app_id| state.app_states.get(&app_id))
        .map(|app_state| app_state.history.clone())
        .unwrap_or_default()
}

pub fn select_selected_state_viewer(state: &State) -> StateViewer {
    state
        .selected_app_id
        .and_then(|app_id| state.app_states.get(&app_id))
        .map(|app_state| app_state.selected_state_viewer)
        .unwrap_or_default()
}

pub fn stream_selected_action_prefix(store: Store) -> ChangesStream<Option<String>> {
    store.changes_transformed(select_selected_change, |s| {
        s.map(|s| extract_action_prefix(&s.action))
    })
}

pub fn stream_selected_action_ron_value(store: Store) -> ChangesStream<Option<ron::Value>> {
    store.changes_transformed(select_selected_change, |s| {
        let s = s?;
        ron::from_str::<ron::Value>(&s.action).ok()
    })
}

pub fn stream_selected_action_ron_pretty(store: Store) -> ChangesStream<Option<String>> {
    store.changes_transformed(select_selected_change, |s| {
        let s = s?;
        ron::ser::to_string_pretty(
            &s.action,
            ron::ser::PrettyConfig::default()
                .indentor(Cow::Borrowed("  "))
                .struct_names(true),
        )
        .ok()
    })
}

pub fn stream_selected_action_json_pretty(store: Store) -> ChangesStream<Option<String>> {
    store.changes_transformed(select_selected_change, |s| {
        let s = s.and_then(|s| ron::from_str::<ron::Value>(&s.action).ok())?;
        serde_json::to_string_pretty(&s).ok()
    })
}

pub fn stream_selected_state_ron_value(store: Store) -> ChangesStream<Option<ron::Value>> {
    store.changes_transformed(select_selected_change, |s| {
        let s = s?;
        ron::from_str::<ron::Value>(&s.state).ok()
    })
}

pub fn stream_selected_state_ron_pretty(store: Store) -> ChangesStream<Option<String>> {
    store.changes_transformed(select_selected_change, |s| {
        let s = s.and_then(|s| ron::from_str::<ron::Value>(&s.state).ok())?;
        to_string_pretty(
            &s,
            ron::ser::PrettyConfig::default()
                .indentor(Cow::Borrowed("  "))
                .struct_names(true),
        )
        .ok()
    })
}

pub fn stream_selected_state_json_pretty(store: Store) -> ChangesStream<Option<String>> {
    store.changes_transformed(select_selected_change, |s| {
        let s = s.and_then(|s| ron::from_str::<ron::Value>(&s.state).ok())?;
        serde_json::to_string_pretty(&s).ok()
    })
}

pub fn stream_selected_change(store: Store) -> ChangesStream<Option<StateChange>> {
    store.changes(select_selected_change)
}

pub fn stream_selected_app_name(store: Store) -> ChangesStream<Option<String>> {
    store.changes(select_selected_app_name)
}

pub fn stream_selected_app_id(store: Store) -> ChangesStream<Option<AppId>> {
    store.changes(select_selected_app_id)
}

pub fn stream_selected_history(store: Store) -> ChangesStream<QueueSync<StateChange>> {
    store.changes(select_selected_history)
}

pub fn stream_selected_history_counters_desc(store: Store) -> ChangesStream<Arc<[GlobalCounter]>> {
    store.changes_transformed(select_selected_history, |s: QueueSync<StateChange>| {
        s.iter()
            .map(|item| item.counter)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    })
}

pub fn stream_app_names(store: Store) -> ChangesStream<Vec<(AppId, String)>> {
    store.changes(select_app_names)
}

pub fn stream_selected_state_viewer(store: Store) -> ChangesStream<StateViewer> {
    store.changes(select_selected_state_viewer)
}

pub fn extract_action_prefix(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if matches!(ch, '(' | '{' | '[') {
            if i + 1 < chars.len() {
                let next_char = chars[i + 1];
                if next_char.is_ascii_lowercase()
                    || next_char == '"'
                    || next_char == '['
                    || next_char == '\n'
                {
                    return (input[..i])
                        .split(&['(', '{', '['])
                        .collect::<Vec<_>>()
                        .join("-");
                }
            }
        } else if matches!(ch, ')' | '}' | ']') {
            return (input[..i])
                .split(&['(', '{', '['])
                .collect::<Vec<_>>()
                .join("-");
        }
    }

    input.to_owned()
}

pub fn select_selected_history_limit(state: &State) -> usize {
    state
        .selected_app_id
        .and_then(|app_id| state.app_states.get(&app_id))
        .map_or(super::DEFAULT_HISTORY_LIMIT, |app_state| {
            app_state.history_limit
        })
}

pub fn select_selected_drop_history_on_reconnect(state: &State) -> bool {
    state
        .selected_app_id
        .and_then(|app_id| state.app_states.get(&app_id))
        .is_some_and(|app_state| app_state.drop_history_on_reconnect)
}

pub fn select_selected_distinct_action_prefixes(state: &State) -> BTreeSet<String> {
    select_selected_history(state)
        .iter()
        .map(|change| extract_action_prefix(&change.action))
        .collect()
}

pub fn stream_selected_history_limit(store: Store) -> ChangesStream<usize> {
    store.changes(select_selected_history_limit)
}

pub fn stream_selected_drop_history_on_reconnect(store: Store) -> ChangesStream<bool> {
    store.changes(select_selected_drop_history_on_reconnect)
}

pub fn stream_selected_distinct_action_prefixes(store: Store) -> ChangesStream<BTreeSet<String>> {
    store.changes(select_selected_distinct_action_prefixes)
}

pub fn select_selected_theme(state: &State) -> String {
    state.selected_theme.clone()
}

pub fn select_themes(state: &State) -> Vec<String> {
    state.themes.iter().cloned().collect()
}

pub fn stream_selected_theme(store: Store) -> ChangesStream<String> {
    store.changes(select_selected_theme)
}

pub fn stream_themes(store: Store) -> ChangesStream<Vec<String>> {
    store.changes(select_themes)
}

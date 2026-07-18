use std::{collections::BTreeSet, fmt::Display, pin::Pin, sync::Arc};

use app_id::AppId;
use bwu_redux::{ActionFilter, StoreConfig, StoreWrapper, devtools_rpc};
use futures::Stream;
use rpds::{HashTrieMapSync, QueueSync, VectorSync};
use themes::THEME_NAMES;
use uuid::Uuid;

use super::{Action, Error, StorageMiddleware};

pub mod app_id;
pub mod themes;

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub errors: VectorSync<Error>,
    pub app_states: HashTrieMapSync<AppId, AppState>,
    pub global_state_counter: GlobalCounter,
    pub selected_app_id: Option<AppId>,
    pub selected_theme: String,
    pub themes: VectorSync<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            errors: VectorSync::<Error>::default(),
            app_states: HashTrieMapSync::<AppId, AppState>::default(),
            global_state_counter: GlobalCounter(0),
            selected_app_id: None,
            selected_theme: String::from("default"),
            themes: ThemeNamesWrapper(THEME_NAMES).into(),
        }
    }
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct AppState {
    pub history: QueueSync<StateChange>,
    pub app_id: AppId,
    pub app_name: String,
    pub selected_state_id: Option<GlobalCounter>,
    pub selected_state_viewer: StateViewer,
    pub selected_theme: String,
    /// Number of history entries kept before the oldest are dropped.
    pub history_limit: usize,
    /// When an app restart is detected (its session counter drops), start
    /// the history fresh instead of appending to what came before.
    pub drop_history_on_reconnect: bool,
    /// Action prefixes (see `extract_action_prefix`) the server should not
    /// forward for this app.
    pub paused_actions: BTreeSet<String>,
}

/// Default number of history entries kept per app before the oldest are
/// dropped.
pub(crate) const DEFAULT_HISTORY_LIMIT: usize = 200;

impl Default for AppState {
    fn default() -> Self {
        Self {
            history: QueueSync::default(),
            app_id: AppId::default(),
            app_name: String::new(),
            selected_state_id: None,
            selected_state_viewer: StateViewer::default(),
            selected_theme: String::from("default"),
            history_limit: DEFAULT_HISTORY_LIMIT,
            drop_history_on_reconnect: false,
            paused_actions: BTreeSet::new(),
        }
    }
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "redux_devtools", serde(transparent))]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct SessionCounter(usize);

impl From<u32> for SessionCounter {
    fn from(value: u32) -> Self {
        Self(value as usize)
    }
}

impl Display for SessionCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "redux_devtools", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct GlobalCounter(usize);

impl GlobalCounter {
    pub fn inc(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn dec(self) -> Self {
        Self(self.0 - 1)
    }

    pub fn into_inner(self) -> usize {
        self.0
    }
}

impl Display for GlobalCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<GlobalCounter> for usize {
    fn from(value: GlobalCounter) -> Self {
        value.0
    }
}

impl From<usize> for GlobalCounter {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StateChange {
    pub counter: GlobalCounter,
    pub session_counter: SessionCounter,
    pub action: String,
    // to avoid state to blow up when devtools are used with devtools
    #[cfg_attr(feature = "redux_devtools", serde(skip_serializing))]
    pub state: String,
}

impl StateChange {
    pub(crate) fn from_message(message: StateChangeMessage, counter: GlobalCounter) -> Self {
        Self {
            counter,
            session_counter: message.session_counter,
            action: message.action,
            state: message.state,
        }
    }
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StateChangeMessage {
    pub(crate) session_counter: SessionCounter,
    pub(crate) action: String,
    pub(crate) state: String,
}

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum StateViewer {
    #[default]
    Tree,
    Json,
    Ron,
}

impl From<devtools_rpc::StateChangeMessage> for StateChangeMessage {
    fn from(value: devtools_rpc::StateChangeMessage) -> Self {
        Self {
            session_counter: value.counter.into(),
            action: value.action,
            state: value.state,
        }
    }
}

struct ThemeNamesWrapper<const N: usize>([&'static str; N]);
impl<const N: usize> From<ThemeNamesWrapper<N>> for VectorSync<String> {
    fn from(value: ThemeNamesWrapper<N>) -> Self {
        value.0.iter().map(|&v| String::from(v)).collect::<Self>()
    }
}

pub type Store = Arc<StoreWrapper<State, Action>>;
pub type ChangesStream<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

pub fn create_store(pause_sink: Arc<dyn super::PauseSink>) -> Store {
    let initial_state = State::default();

    Arc::new(StoreWrapper::new(
        StoreConfig::new(initial_state, super::reducer)
            .with_middleware(vec![
                Arc::new(StorageMiddleware),
                Arc::new(super::PauseMiddleware::new(pause_sink)),
            ])
            .with_history_size(100)
            // On wasm the GUI is a devtools *viewer* (see `devtools_watch`);
            // sending its own state to the hard-coded devtools URL would be
            // blocked as mixed content on the https deployment anyway.
            .with_devtools(cfg!(not(target_family = "wasm")))
            .with_devtools_action_filter(Box::new(DevToolsActionFilter))
            .with_app_id(
                Uuid::parse_str("eb7a7474-54f1-11f0-9f3f-52540081f304").expect("To always succeed"),
            )
            .with_app_name("BWU Redux DevTools"),
    ))
}

#[derive(Copy, Clone, Debug, Default)]
pub struct DevToolsActionFilter;

impl ActionFilter<Action> for DevToolsActionFilter {
    fn filter(&self, action: &Action) -> bool {
        !matches!(action, Action::StateUpdate { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_store_minimal() {
        let x = create_store(Arc::new(super::super::NoopPauseSink));
        assert_eq!(x.select(std::clone::Clone::clone), State::default());
    }
}

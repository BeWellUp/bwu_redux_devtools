//! Broadcast hub connecting `StateChange` producers with `Watch` stream
//! subscribers, keeping a per-app replay buffer for late joiners.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Mutex, PoisonError},
};

use bwu_redux::devtools_rpc::{self, StateChangeMessage, StateChangeRequest};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::redux::selectors::extract_action_prefix;

/// Capacity of the live broadcast channel to `Watch` subscribers.
const BROADCAST_CAPACITY: usize = 256;
/// Per-app replay buffer size; matches the GUI's `MAX_HISTORY_ENTRIES`.
const MAX_BUFFERED_CHANGES: usize = 200;

#[derive(Debug)]
pub(crate) struct WatchHub {
    inner: Mutex<HubInner>,
}

#[derive(Debug)]
struct HubInner {
    tx: broadcast::Sender<StateChangeRequest>,
    buffer: HashMap<Uuid, AppBuffer>,
    /// Action prefixes (see `extract_action_prefix`) to drop for a given
    /// app, set via the `SetPause` RPC. Kept separate from `buffer` so a
    /// pause can be set before the app's first `StateChange` arrives.
    paused: HashMap<Uuid, HashSet<String>>,
}

#[derive(Debug)]
struct AppBuffer {
    app_name: String,
    changes: VecDeque<StateChangeMessage>,
}

impl WatchHub {
    pub(crate) fn new() -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            inner: Mutex::new(HubInner {
                tx,
                buffer: HashMap::new(),
                paused: HashMap::new(),
            }),
        }
    }

    /// Record `req` in the replay buffer and forward it to live subscribers,
    /// dropping any changes whose action prefix is currently paused for this
    /// app. Returns the changes that were actually kept (buffered and
    /// broadcast), so callers needing the same filtered view (e.g. the
    /// embedded desktop GUI's own store) don't have to duplicate the filter.
    pub(crate) fn publish(
        &self,
        app_id: Uuid,
        req: &StateChangeRequest,
    ) -> Vec<StateChangeMessage> {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);

        let kept: Vec<StateChangeMessage> = inner.paused.get(&app_id).map_or_else(
            || req.changes.clone(),
            |paused| {
                req.changes
                    .iter()
                    .filter(|change| !paused.contains(&extract_action_prefix(&change.action)))
                    .cloned()
                    .collect()
            },
        );

        let app_buffer = inner.buffer.entry(app_id).or_insert_with(|| AppBuffer {
            app_name: req.app_name.clone(),
            changes: VecDeque::new(),
        });
        app_buffer.app_name.clone_from(&req.app_name);
        app_buffer.changes.extend(kept.iter().cloned());
        while app_buffer.changes.len() > MAX_BUFFERED_CHANGES {
            let _ = app_buffer.changes.pop_front();
        }

        if !kept.is_empty() {
            let filtered_req = StateChangeRequest {
                changes: kept.clone(),
                ..req.clone()
            };
            // A send error only means there are no subscribers right now.
            let _ = inner.tx.send(filtered_req);
        }

        kept
    }

    /// Replace the set of paused action prefixes for `app_id`; an empty set
    /// un-pauses. Does not retroactively remove already-buffered changes.
    pub(crate) fn set_pause(&self, app_id: Uuid, paused_action_prefixes: HashSet<String>) {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if paused_action_prefixes.is_empty() {
            let _ = inner.paused.remove(&app_id);
        } else {
            let _ = inner.paused.insert(app_id, paused_action_prefixes);
        }
    }

    /// Subscribe to live changes and get the buffered history for replay.
    ///
    /// Both the snapshot and the subscription happen under the same lock as
    /// `publish`, so a subscriber can neither miss nor double-receive a
    /// change between replay and the live stream.
    pub(crate) fn subscribe(
        &self,
    ) -> (
        Vec<StateChangeRequest>,
        broadcast::Receiver<StateChangeRequest>,
    ) {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let rx = inner.tx.subscribe();
        let replay = inner
            .buffer
            .iter()
            .map(|(app_id, app_buffer)| StateChangeRequest {
                app_id: Some(devtools_rpc::Uuid {
                    value: app_id.to_string(),
                }),
                app_name: app_buffer.app_name.clone(),
                changes: app_buffer.changes.iter().cloned().collect(),
                replay: true,
            })
            .collect();
        (replay, rx)
    }
}

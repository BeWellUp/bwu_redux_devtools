//! Broadcast hub connecting `StateChange` producers with `Watch` stream
//! subscribers, keeping a per-app replay buffer for late joiners.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, PoisonError},
};

use bwu_redux::devtools_rpc::{self, StateChangeMessage, StateChangeRequest};
use tokio::sync::broadcast;
use uuid::Uuid;

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
            }),
        }
    }

    /// Record `req` in the replay buffer and forward it to live subscribers.
    pub(crate) fn publish(&self, app_id: Uuid, req: &StateChangeRequest) {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);

        let app_buffer = inner.buffer.entry(app_id).or_insert_with(|| AppBuffer {
            app_name: req.app_name.clone(),
            changes: VecDeque::new(),
        });
        app_buffer.app_name.clone_from(&req.app_name);
        app_buffer.changes.extend(req.changes.iter().cloned());
        while app_buffer.changes.len() > MAX_BUFFERED_CHANGES {
            let _ = app_buffer.changes.pop_front();
        }

        // A send error only means there are no subscribers right now.
        let _ = inner.tx.send(req.clone());
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

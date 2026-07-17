//! Watch client for the web (WASM) GUI: subscribes to a devtools server's
//! `Watch` stream via gRPC-web and feeds received state changes into the
//! local store.

use std::{collections::HashMap, time::Duration};

use bwu_redux::devtools_rpc::{StateChangeRequest, WatchRequest, dev_tools_client::DevToolsClient};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

use crate::redux::Action;

/// Initial delay between reconnection attempts; doubles on each failure
/// (exponential backoff) up to [`MAX_RETRY_DELAY`].
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
/// Maximum backoff delay between reconnection attempts.
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);

/// Devtools server URL used when the page is served directly by `dx serve`
/// (no reverse proxy in front). The server's CORS is fully open, so the
/// cross-origin gRPC-web connection works.
const DEV_FALLBACK_URL: &str = "http://localhost:49051";

/// Determine the devtools server URL at runtime.
///
/// When served behind a reverse proxy (Caddy) the gRPC-web endpoints are
/// exposed same-origin, so the page origin is used verbatim. A plain
/// `localhost`/`127.*` origin means `dx serve` development mode, where the
/// devtools server runs separately on its default port.
fn server_url() -> String {
    let location = web_sys::window().map(|window| window.location());
    let hostname = location
        .as_ref()
        .and_then(|location| location.hostname().ok());
    let origin = location
        .as_ref()
        .and_then(|location| location.origin().ok());

    match (hostname, origin) {
        (Some(hostname), Some(origin))
            if hostname != "localhost" && !hostname.starts_with("127.") =>
        {
            origin
        }
        _ => DEV_FALLBACK_URL.to_owned(),
    }
}

/// Connect to the devtools server, dispatch received state changes into the
/// store, and reconnect with exponential backoff when the stream ends.
#[expect(
    clippy::future_not_send,
    reason = "wasm-only code; futures run on the single-threaded browser event loop"
)]
pub async fn run(dispatch_tx: UnboundedSender<Action>) {
    let url = server_url();
    info!("Devtools watch client connecting to {url}");

    // Highest counter seen per app (proto UUID string), used to drop
    // already-known entries from history replayed after a reconnect.
    let mut max_seen: HashMap<String, u32> = HashMap::new();
    let mut delay = INITIAL_RETRY_DELAY;

    loop {
        match watch(&url, &mut max_seen, &dispatch_tx).await {
            Ok(()) => {
                delay = INITIAL_RETRY_DELAY;
                info!("Devtools watch stream ended; reconnecting");
            }
            Err(err) => {
                warn!("Devtools watch stream failed (retrying in {delay:?}): {err}");
            }
        }

        futures_timer::Delay::new(delay).await;
        delay = std::cmp::min(delay.saturating_mul(2), MAX_RETRY_DELAY);
    }
}

#[expect(
    clippy::future_not_send,
    reason = "wasm-only code; futures run on the single-threaded browser event loop"
)]
async fn watch(
    url: &str,
    max_seen: &mut HashMap<String, u32>,
    dispatch_tx: &UnboundedSender<Action>,
) -> Result<(), tonic::Status> {
    let mut client = DevToolsClient::new(tonic_web_wasm_client::Client::new(url.to_owned()));
    let mut stream = client.watch(WatchRequest {}).await?.into_inner();

    while let Some(msg) = stream.message().await? {
        forward(msg, max_seen, dispatch_tx);
    }

    Ok(())
}

/// Dispatch a received message into the store, dropping replayed entries
/// that are already known.
///
/// Live messages are always dispatched and *set* the per-app high-water
/// mark (a lower counter means the monitored app restarted); replayed
/// messages only *raise* it.
fn forward(
    msg: StateChangeRequest,
    max_seen: &mut HashMap<String, u32>,
    dispatch_tx: &UnboundedSender<Action>,
) {
    let Some(ref app_id) = msg.app_id else {
        warn!("Ignoring devtools message without app_id");
        return;
    };
    let app_key = app_id.value.clone();
    let seen = max_seen.get(&app_key).copied();

    let changes = if msg.replay {
        msg.changes
            .iter()
            .filter(|change| seen.is_none_or(|seen| change.counter > seen))
            .cloned()
            .collect()
    } else {
        msg.changes.clone()
    };
    if changes.is_empty() {
        return;
    }

    if let Some(message_max) = changes.iter().map(|change| change.counter).max() {
        if msg.replay {
            let entry = max_seen.entry(app_key).or_insert(message_max);
            *entry = (*entry).max(message_max);
        } else {
            let _previous = max_seen.insert(app_key, message_max);
        }
    }

    match Action::try_from(StateChangeRequest { changes, ..msg }) {
        Ok(action) => {
            if let Err(err) = dispatch_tx.send(action) {
                warn!("dispatching action StateUpdate failed: {err}");
            }
        }
        Err(err) => warn!("Ignoring invalid devtools message: {err}"),
    }
}

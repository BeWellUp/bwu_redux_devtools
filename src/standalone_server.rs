use bwu_redux_devtools::devtools_server::server::DevtoolsServer;
use tracing_subscriber::prelude::*;

#[tokio::main]
pub(crate) async fn main() {
    let registry = tracing_subscriber::registry();

    // tokio-console support; additionally requires
    // RUSTFLAGS="--cfg tokio_unstable" at build time.
    #[cfg(feature = "tokio-console")]
    let registry = registry.with(console_subscriber::spawn());

    registry
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .without_time()
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
        )
        .init();

    // No local store: received changes are buffered and re-broadcast to
    // `Watch` stream subscribers (e.g. the web GUI) by the server itself.
    let server = DevtoolsServer::new(None);
    if server.run().await.is_err() {
        std::process::exit(1);
    }
}

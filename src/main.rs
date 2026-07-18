use std::sync::Arc;

use bwu_redux_devtools::redux::{
    Action, ReduxStateChange, create_store, selectors::stream_selected_theme,
};
use dioxus::prelude::*;
use futures::StreamExt as _;
use pause_controller::PauseController;
use route::Route;

pub(crate) mod components;
pub(crate) mod pause_controller;
pub(crate) mod route;

#[cfg(not(target_family = "wasm"))]
use bwu_redux_devtools::devtools_server::server::DevtoolsServer;

pub(crate) mod views;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Created hub-first (no dispatch sender yet) so it can back the pause
    // controller the store needs *during construction*; `.with_dispatch_tx`
    // below hands the same hub a working sender once the store exists,
    // without losing any pause state set on it in between.
    #[cfg(not(target_family = "wasm"))]
    let hub_server = use_hook(|| DevtoolsServer::new(None));
    #[cfg(not(target_family = "wasm"))]
    let hub_server_for_pause = hub_server.clone();

    let store = use_context_provider(move || {
        #[cfg(not(target_family = "wasm"))]
        let pause_controller = PauseController::embedded(hub_server_for_pause);
        #[cfg(target_family = "wasm")]
        let pause_controller = PauseController::remote();

        let store = create_store(Arc::new(pause_controller));
        store.run();
        store
    });

    let _ = store.dispatch(Action::ReduxStateChange(ReduxStateChange::StoreInit));

    // Apply the selected DaisyUI theme on the document root (persisted via
    // StorageMiddleware; "default" falls back to light/dark by system
    // preference). Theme names come from the fixed THEME_NAMES list, so the
    // eval interpolation is safe.
    let mut selected_theme: Signal<String> = use_signal(|| String::from("default"));
    let theme_store = store.clone();
    let _ = use_resource(move || {
        let store = theme_store.clone();
        async move {
            let mut stream = stream_selected_theme(store);

            while let Some(value) = stream.next().await {
                selected_theme.set(value);
            }
        }
    });
    let _ = use_effect(move || {
        let theme = selected_theme();
        let _ = document::eval(&format!(
            "document.documentElement.setAttribute('data-theme', '{theme}');"
        ));
    });

    let dispatch_sender = store.get_dispatch_sender();
    #[cfg(not(target_family = "wasm"))]
    let _devtools_server_future = use_future(move || {
        let server = hub_server.with_dispatch_tx(dispatch_sender.clone());
        async move {
            let _ = server.run().await;
        }
    });

    #[cfg(all(target_family = "wasm", feature = "redux_devtools"))]
    let _devtools_watch_future = use_future(move || {
        let dispatch_sender = dispatch_sender.clone();
        async move {
            bwu_redux_devtools::devtools_watch::run(dispatch_sender).await;
        }
    });

    rsx! {
        document::Title { "BWU Redux DevTools" }
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: TAILWIND_CSS }

        Router::<Route> {}
    }
}

use dioxus::prelude::*;

use crate::views::{AppStateView, HomeView, NoAppSelected, NotFoundView};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub(crate) enum Route {
    // Redirect root before any component renders, so use_route never sees "/"
    #[redirect("/", || Route::NoAppSelected)]

    #[nest("/data")]
      #[layout(HomeView)]
        // Explicit unambiguous path avoids trailing-slash parsing edge cases
        #[route("/home")]
        NoAppSelected,
        #[route("/app/:app_id")]
        AppStateView {app_id: String},

      #[end_layout]
    #[end_nest]


    #[route("/:..segments")]
    NotFoundView {
        segments: Vec<String>
    },
}

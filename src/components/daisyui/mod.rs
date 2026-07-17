//! DaisyUI-styled components vendored from the internal `bwu_dioxus_components`
//! crate of the bwu_app workspace. `tooltip` is
//! interim and get replaced by official Dioxus components; `drawer`, `menu`,
//! and `kbd` have no official equivalent and stay vendored.

mod drawer;
mod kbd;
mod menu;

pub(crate) use drawer::*;
pub(crate) use kbd::*;
pub(crate) use menu::*;

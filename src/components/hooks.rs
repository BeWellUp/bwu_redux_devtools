use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::prelude::*;

/// from <https://github.com/DioxusLabs/components/blob/main/primitives/src/lib.rs>
/// Generate a runtime-unique id.
#[must_use]
#[inline]
pub(crate) fn use_unique_id() -> Signal<String> {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    let initial_value = use_hook(|| {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let id_str = format!("dxc-{id}");
        id_str
    });

    use_signal(|| initial_value)
}

// Elements can only have one id so if the user provides their own, we must use it as the aria id.
#[must_use]
#[inline]
pub(crate) fn use_id_or<T: Clone + PartialEq + 'static>(
    mut gen_id: Signal<T>,
    user_id: ReadSignal<Option<T>>,
) -> Memo<T> {
    // First, check if we have a user-provided ID
    let has_user_id = use_memo(move || user_id().is_some());

    // If we have a user ID, update the gen_id in an effect
    #[expect(unused_results, reason = "We don't need the result")]
    use_effect(move || {
        if let Some(id) = user_id() {
            gen_id.set(id);
        }
    });

    // Return the appropriate ID
    use_memo(move || {
        if has_user_id() {
            user_id().unwrap()
        } else {
            gen_id.peek().clone()
        }
    })
}

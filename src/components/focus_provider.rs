use std::sync::Arc;

use tokio::sync::Mutex;

#[derive(Clone, Debug, Default)]
pub(crate) struct FocusProvider {
    focus: Arc<Mutex<Vec<String>>>,
}

impl FocusProvider {
    #[inline]
    pub(crate) async fn add(&self, scope: String) {
        let mut guard = self.focus.lock().await;
        guard.push(scope);
    }

    #[inline]
    pub(crate) async fn remove(&self, scope: String) {
        let mut guard = self.focus.lock().await;
        (*guard).retain(|v| *v != scope);
    }
}

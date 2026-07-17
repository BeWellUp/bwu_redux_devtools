use std::fmt::Display;

use uuid::Uuid;

#[cfg_attr(
    feature = "redux_devtools",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "redux_devtools", serde(transparent))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct AppId(Uuid);

impl AppId {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self(Uuid::now_v7())
    }

    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for AppId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for AppId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl AsRef<Uuid> for AppId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

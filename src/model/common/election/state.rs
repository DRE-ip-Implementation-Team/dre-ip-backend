use mongodb::bson::{to_bson, Bson};
use serde::{Deserialize, Serialize};

/// States in the Election lifecycle.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElectionState {
    /// Under construction, only visible to admins.
    Draft,
    /// Ready, in progress, or completed. Visible to all.
    Published,
    /// Completed, hidden by default, but retrievable by all.
    Archived,
}

impl From<ElectionState> for Bson {
    fn from(state: ElectionState) -> Self {
        to_bson(&state).expect("Serialisation is infallible")
    }
}

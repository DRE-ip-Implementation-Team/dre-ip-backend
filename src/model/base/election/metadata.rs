use chrono::{DateTime, Utc};
use mongodb::bson::{serde_helpers::chrono_datetime_as_bson_datetime, to_bson, Bson};
use serde::{Deserialize, Serialize};

/// A view on just the election's top-level metadata.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionMetadata {
    /// Election name.
    pub name: String,
    /// Election state.
    pub state: ElectionState,
    /// Election start time.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub start_time: DateTime<Utc>,
    /// Election end time.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub end_time: DateTime<Utc>,
}

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

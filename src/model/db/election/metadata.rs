use chrono::{DateTime, Utc};
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

use crate::model::common::election::ElectionState;

/// A view on just the election's top-level metadata.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
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

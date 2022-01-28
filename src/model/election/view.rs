use chrono::{DateTime, Utc};
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionView {
    name: String,
    finalised: bool,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    start_time: DateTime<Utc>,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    end_time: DateTime<Utc>,
}

impl ElectionView {
    pub fn new(
        name: String,
        finalised: bool,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            name,
            finalised,
            start_time,
            end_time,
        }
    }
}

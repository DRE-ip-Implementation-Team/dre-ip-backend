use mongodb::{
    bson::doc,
    options::{FindOneAndUpdateOptions, ReturnDocument},
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::mongodb::{Coll, Id};

/// A counter object used to implement auto-increment fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    #[serde(rename = "_id")]
    pub id: Id,
    pub next: u64,
}

impl Counter {
    /// Create a new `Counter` starting at the given value, optionally specifying the ID to use.
    pub fn new(id: impl Into<Option<Id>>, start: u64) -> Self {
        let id = id.into().unwrap_or_else(Id::new);
        Self { id, next: start }
    }

    /// Atomically retrieve the next value of the counter with the given ID.
    pub async fn next(counters: &Coll<Counter>, id: Id) -> Result<u64> {
        let update = doc! {
            "$inc": { "next": 1 }
        };
        let options: FindOneAndUpdateOptions = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::Before)
            .build();
        let counter = counters
            .find_one_and_update(id.as_doc(), update, options)
            .await?
            .ok_or_else(|| {
                Error::Status(
                    Status::InternalServerError,
                    format!("Failed to find counter with ID {}", id),
                )
            })?;
        Ok(counter.next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use mongodb::Database;

    #[backend_test]
    async fn counter_increment(db: Database) {
        const START: u64 = 5;

        // Create a counter and insert it.
        let counter = Counter::new(None, START);
        let counters = Coll::<Counter>::from_db(&db);
        let id: Id = counters
            .insert_one(counter, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
            .into();

        // Get the next value.
        let next = Counter::next(&counters, id).await.unwrap();
        assert_eq!(next, START);

        // Check the counter was incremented.
        let counter = counters.find_one(id.as_doc(), None).await.unwrap().unwrap();
        assert_eq!(counter.next, START + 1);
    }
}

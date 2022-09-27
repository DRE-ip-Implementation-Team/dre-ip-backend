use mongodb::{
    bson::doc,
    error::Error as DbError,
    options::{FindOneAndUpdateOptions, ReturnDocument, UpdateOptions},
};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::model::{
    common::election::{ElectionId, QuestionId},
    mongodb::Coll,
};

/// The unique ID of the counter for election IDs.
pub const ELECTION_ID_COUNTER_ID: &str = "eid";

/// Get the unique ID for the ballot counter for the given question.
pub fn ballot_counter_id(election_id: ElectionId, question_id: QuestionId) -> String {
    format!("bid:{}:{}", election_id, question_id)
}

/// A counter object used to implement auto-increment fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    #[serde(rename = "_id")]
    pub id: String,
    pub next: u32,
}

impl Counter {
    /// Atomically retrieve the next value of the counter with the given ID.
    pub async fn next(counters: &Coll<Self>, id: &str) -> Result<u32, Error> {
        Self::reserve(counters, id, 1).await
    }

    /// Reserve `count` unique IDs, starting at the returned value.
    pub async fn reserve(counters: &Coll<Self>, id: &str, count: u32) -> Result<u32, Error> {
        let filter = doc! {
            "_id": id
        };
        let update = doc! {
            "$inc": { "next": count }
        };
        let options: FindOneAndUpdateOptions = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::Before)
            .build();
        let counter = counters
            .find_one_and_update(filter, update, options)
            .await?
            .ok_or_else(|| {
                Error::Status(
                    Status::InternalServerError,
                    format!("Failed to find counter with ID `{}`", id),
                )
            })?;
        Ok(counter.next)
    }
}

/// Create the global election ID counter if it does not already exist.
pub async fn ensure_election_id_counter_exists(counters: &Coll<Counter>) -> Result<(), DbError> {
    let filter = doc! {
        "_id": ELECTION_ID_COUNTER_ID,
    };
    let update = doc! {
        "$setOnInsert": {
            "_id": ELECTION_ID_COUNTER_ID,
            "next": 1,
        }
    };
    let options: UpdateOptions = UpdateOptions::builder().upsert(true).build();
    counters.update_one(filter, update, options).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use mongodb::Database;

    #[backend_test]
    async fn counter_increment(db: Database) {
        const START: u32 = 5;
        const ID: &str = "unique";

        // Create a counter and insert it.
        let counter = Counter {
            id: ID.to_string(),
            next: START,
        };
        let counters = Coll::<Counter>::from_db(&db);
        counters.insert_one(counter, None).await.unwrap();

        // Get the next value.
        let next = Counter::next(&counters, ID).await.unwrap();
        assert_eq!(next, START);

        // Check the counter was incremented.
        let counter = counters
            .find_one(doc! {"_id": ID}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(counter.next, START + 1);
    }
}

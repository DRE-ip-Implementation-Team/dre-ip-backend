use std::ops::Deref;

use mongodb::{
    bson::doc, error::Error as DbError, options::IndexOptions, Collection, Database, IndexModel,
};
use rocket::{
    request::{self, FromRequest, Request},
    State,
};

use crate::model::{
    common::ballot::BallotState,
    db::{
        admin::{Admin, NewAdmin},
        ballot::{AnyBallot, Ballot, BallotCore},
        candidate_totals::{CandidateTotals, NewCandidateTotals},
        election::{Election, ElectionMetadata},
        voter::{NewVoter, Voter},
    },
};

use super::counter::Counter;

/// A type that can be directly inserted/read to/from the database.
pub trait MongoCollection {
    /// The name of the collection.
    const NAME: &'static str;
}

/// A database collection of the given type.
pub struct Coll<T>(Collection<T>);

impl<T> Coll<T>
where
    T: MongoCollection,
{
    /// Get a handle on this collection in the given database.
    pub fn from_db(db: &Database) -> Self {
        Self(db.collection(T::NAME))
    }
}

// `Derive(Clone)` would only derive if `T: Clone`, but we don't need that bound.
impl<T> Clone for Coll<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for Coll<T> {
    type Target = Collection<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'r, T> FromRequest<'r> for Coll<T>
where
    T: MongoCollection,
{
    type Error = ();

    /// Get the database connection from the managed state and wrap it in a collection.
    ///
    /// Panics iff the [`Database`] is not managed by [`rocket::Rocket`].
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let db = req.guard::<&State<Database>>().await.unwrap();
        request::Outcome::Success(Coll::from_db(db))
    }
}

// Admin collections
const ADMINS: &str = "admins";
impl MongoCollection for Admin {
    const NAME: &'static str = ADMINS;
}
impl MongoCollection for NewAdmin {
    const NAME: &'static str = ADMINS;
}

// Voter collections
const VOTERS: &str = "voters";
impl MongoCollection for Voter {
    const NAME: &'static str = VOTERS;
}
impl MongoCollection for NewVoter {
    const NAME: &'static str = VOTERS;
}

// Election collections
const ELECTIONS: &str = "elections";
impl MongoCollection for Election {
    const NAME: &'static str = ELECTIONS;
}
impl MongoCollection for ElectionMetadata {
    const NAME: &'static str = ELECTIONS;
}

// Ballot collections
const BALLOTS: &str = "ballots";
impl<S: BallotState> MongoCollection for BallotCore<S> {
    const NAME: &'static str = BALLOTS;
}
impl<S: BallotState> MongoCollection for Ballot<S> {
    const NAME: &'static str = BALLOTS;
}
impl MongoCollection for AnyBallot {
    const NAME: &'static str = BALLOTS;
}

// Candidate totals collections
const CANDIDATE_TOTALS: &str = "candidate_totals";
impl MongoCollection for CandidateTotals {
    const NAME: &'static str = CANDIDATE_TOTALS;
}
impl MongoCollection for NewCandidateTotals {
    const NAME: &'static str = CANDIDATE_TOTALS;
}

// Counter collection
const COUNTERS: &str = "counters";
impl MongoCollection for Counter {
    const NAME: &'static str = COUNTERS;
}

/// Ensure that all the required indexes exist on the given database.
///
/// This operation is idempotent.
pub async fn ensure_indexes_exist(db: &Database) -> Result<(), DbError> {
    debug!("Ensuring collection indexes exist");

    let unique = IndexOptions::builder().unique(true).build();

    // Voter collection.
    let voter_index = IndexModel::builder()
        .keys(doc! {"sms_hmac": 1})
        .options(unique.clone())
        .build();
    Coll::<Voter>::from_db(db)
        .create_index(voter_index, None)
        .await?;

    // Admin collection.
    let admin_index = IndexModel::builder()
        .keys(doc! {"username": 1})
        .options(unique.clone())
        .build();
    Coll::<Admin>::from_db(db)
        .create_index(admin_index, None)
        .await?;

    // Ballot collection.
    let ballot_index = IndexModel::builder()
        .keys(doc! {"election_id": 1, "question_id": 1, "ballot_id": 1})
        .options(unique.clone())
        .build();
    Coll::<AnyBallot>::from_db(db)
        .create_index(ballot_index, None)
        .await?;

    // Candidate totals collection.
    let totals_index = IndexModel::builder()
        .keys(doc! {"election_id": 1, "question_id": 1, "candidate_name": 1})
        .options(unique.clone())
        .build();
    Coll::<CandidateTotals>::from_db(db)
        .create_index(totals_index, None)
        .await?;

    Ok(())
}

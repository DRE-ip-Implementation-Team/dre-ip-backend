use std::ops::Deref;

use mongodb::{Collection, Database};
use rocket::{
    request::{self, FromRequest, Request},
    State,
};

use crate::model::db::{
    admin::{Admin, NewAdmin},
    ballot::{Ballot, BallotState, FinishedBallot},
    candidate_totals::{CandidateTotals, NewCandidateTotals},
    election::{Election, ElectionMetadata, NewElection},
    voter::{NewVoter, Voter},
};

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
impl MongoCollection for NewElection {
    const NAME: &'static str = ELECTIONS;
}

// Ballot collections
const BALLOTS: &str = "ballots";
impl<S: BallotState> MongoCollection for Ballot<S> {
    const NAME: &'static str = BALLOTS;
}
impl MongoCollection for FinishedBallot {
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

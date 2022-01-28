use std::ops::Deref;

use mongodb::{Collection, Database};
use rocket::{
    request::{self, FromRequest, Request},
    State,
};

use crate::model::{
    admin::{db::DbAdmin, Admin},
    ballot::{db::DbBallot, Ballot},
    election::{db::DbElection, view::ElectionView, Election, ElectionSpec},
    voter::{db::DbVoter, Voter},
};

pub trait MongoCollection {
    fn collection_name() -> &'static str;
}

pub struct Coll<T>(Collection<T>);

impl<T> Coll<T>
where
    T: MongoCollection,
{
    pub fn from_db(db: &Database) -> Self {
        Self(db.collection(T::collection_name()))
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

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let db = req.guard::<&State<Database>>().await.unwrap();
        request::Outcome::Success(Coll::from_db(db))
    }
}

impl MongoCollection for Admin {
    fn collection_name() -> &'static str {
        "admins"
    }
}

impl MongoCollection for DbAdmin {
    fn collection_name() -> &'static str {
        "admins"
    }
}

impl MongoCollection for Voter {
    fn collection_name() -> &'static str {
        "voters"
    }
}

impl MongoCollection for DbVoter {
    fn collection_name() -> &'static str {
        "voters"
    }
}

impl MongoCollection for Election {
    fn collection_name() -> &'static str {
        "elections"
    }
}

impl MongoCollection for DbElection {
    fn collection_name() -> &'static str {
        "elections"
    }
}

impl MongoCollection for ElectionView {
    fn collection_name() -> &'static str {
        "elections"
    }
}

impl MongoCollection for ElectionSpec {
    fn collection_name() -> &'static str {
        "elections"
    }
}

impl MongoCollection for Ballot {
    fn collection_name() -> &'static str {
        "ballots"
    }
}

impl MongoCollection for DbBallot {
    fn collection_name() -> &'static str {
        "ballots"
    }
}

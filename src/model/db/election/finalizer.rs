use std::collections::HashMap;

use mongodb::{bson::doc, error::Error as DbError, Database};
use rocket::futures::TryStreamExt;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::Status,
    tokio::sync::Mutex,
    Build, Rocket,
};
use std::sync::Arc;

use crate::{
    error::Error,
    model::{
        common::{
            ballot::{Audited, Unconfirmed},
            election::{ElectionId, ElectionState},
        },
        db::{ballot::Ballot, election::Election},
        mongodb::Coll,
    },
    scheduled_task::ScheduledTask,
};

/// Election finalizers: scheduled tasks for auditing unconfirmed ballots at the end of an election.
pub struct RawElectionFinalizers(pub HashMap<ElectionId, ScheduledTask<Result<(), Error>>>);

/// `ElectionFinalizers` are always accessed behind an Arc-Mutex for thread safety.
pub type ElectionFinalizers = Arc<Mutex<RawElectionFinalizers>>;

impl RawElectionFinalizers {
    /// Create an empty set of election finalizers.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Schedule a finalizer for every published and archived election.
    pub async fn schedule_elections(&mut self, db: &Database) -> Result<(), DbError> {
        // Get all the relevant elections.
        let filter = doc! {
            "$or": [{"state": ElectionState::Published}, {"state": ElectionState::Archived}],
        };
        let all_elections: Vec<_> = Coll::<Election>::from_db(db)
            .find(filter, None)
            .await?
            .try_collect()
            .await?;
        // Add all of them.
        for election in all_elections {
            let unconfirmed_ballots = Coll::<Ballot<Unconfirmed>>::from_db(db);
            let audited_ballots = Coll::<Ballot<Audited>>::from_db(db);
            self.schedule_election(unconfirmed_ballots, audited_ballots, &election);
        }

        Ok(())
    }

    /// Schedule a finalizer for the given election.
    pub fn schedule_election(
        &mut self,
        unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
        audited_ballots: Coll<Ballot<Audited>>,
        election: &Election,
    ) {
        let finalizer = Self::finalizer(election.id, unconfirmed_ballots, audited_ballots);
        // Schedule the finalizer and keep track of it.
        let finalizer_task = ScheduledTask::new(finalizer, election.metadata.end_time);
        self.0.insert(election.id, finalizer_task);
    }

    /// Immediately trigger the finalizer for the given election.
    /// If the finalizer was not previously scheduled, this will have no effect.
    pub async fn finalize_election(&mut self, election_id: ElectionId) -> Result<(), Error> {
        match self.0.remove(&election_id) {
            Some(finalizer) => {
                finalizer.trigger_now();
                finalizer.await.unwrap_or_else(|_| {
                    Err(Error::Status(
                        Status::InternalServerError,
                        format!("Failed to finalize election {}", election_id),
                    ))
                })
            }
            None => Ok(()),
        }
    }

    /// Finalize the given election by auditing all unconfirmed ballots.
    async fn finalizer(
        election_id: ElectionId,
        unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
        audited_ballots: Coll<Ballot<Audited>>,
    ) -> Result<(), Error> {
        /// Nested function for error handling.
        async fn finalize(
            election_id: ElectionId,
            unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
            audited_ballots: Coll<Ballot<Audited>>,
        ) -> Result<(), Error> {
            debug!("Running finalizer for election {election_id}");
            // Get all unconfirmed ballots.
            let filter = doc! {
                "election_id": election_id,
                "state": Unconfirmed,
            };
            let ballots: Vec<_> = unconfirmed_ballots
                .find(filter, None)
                .await?
                .try_collect()
                .await?;
            // Audit them. Bulk operation support isn't in rust-mongodb yet,
            // so we have to do them individually.
            // We deliberately do not do this in a transaction as a partial
            // audit is still better than nothing.
            let num_ballots = ballots.len();
            for ballot in ballots {
                let ballot = ballot.audit();
                let result = audited_ballots
                    .replace_one(ballot.internal_id.as_doc(), &ballot, None)
                    .await?;
                assert_eq!(result.modified_count, 1);
            }
            if num_ballots > 0 {
                warn!("Finalized election {election_id}, audited {num_ballots} ballots");
            } else {
                debug!("Finalizer for election {election_id} had nothing to do");
            }
            Ok(())
        }

        let result = finalize(election_id, unconfirmed_ballots, audited_ballots).await;
        if let Err(ref e) = result {
            error!("Finalizer for election {election_id} failed, unconfirmed ballots might be leaked: {e}");
            error!("Failed finalizer will be retried on next server boot");
            // TODO: retry automatically
        }
        result
    }
}

impl Default for RawElectionFinalizers {
    fn default() -> Self {
        Self::new()
    }
}

/// A fairing that schedules finalizers for all applicable elections
/// during Rocket ignition, and places an `ElectionFinalizers` into managed state.
/// This fairing depends on the database being available in managed state,
/// and so must be attached after the fairing responsible for that.
pub struct ElectionFinalizerFairing;

#[rocket::async_trait]
impl Fairing for ElectionFinalizerFairing {
    fn info(&self) -> Info {
        Info {
            name: "Election Finalizers",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        // Create an election finalizer for every election that needs one.
        info!("Scheduling election finalizers...");
        let mut election_finalizers = RawElectionFinalizers::new();
        let db = match rocket.state::<Database>() {
            Some(db) => db,
            None => {
                error!("Database was not available when scheduling finalizers");
                return Err(rocket);
            }
        };
        if let Err(e) = election_finalizers.schedule_elections(db).await {
            error!("Failed to schedule election finalizers: {e}");
            return Err(rocket);
        }
        info!("...election finalizers scheduled!");

        // Manage the state.
        rocket = rocket.manage(Arc::new(Mutex::new(election_finalizers)));
        Ok(rocket)
    }
}

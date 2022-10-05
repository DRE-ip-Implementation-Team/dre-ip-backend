use chrono::{Duration, Utc};
use mongodb::{bson::doc, error::Error as DbError, Database};
use rocket::futures::TryStreamExt;
use rocket::{
    fairing::{Fairing, Info, Kind},
    futures::future::{BoxFuture, FutureExt},
    http::Status,
    tokio::sync::Mutex,
    Build, Rocket,
};
use std::collections::HashMap;
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

/// Map from election IDs to finalizer tasks.
type TaskMap = HashMap<ElectionId, ScheduledTask<Result<(), Error>>>;

/// Election finalizers: scheduled tasks for auditing unconfirmed ballots at the end of an election.
pub struct ElectionFinalizers {
    tasks: Arc<Mutex<TaskMap>>,
}

impl ElectionFinalizers {
    /// Create an empty set of election finalizers.
    pub fn new() -> Self {
        Self {
            tasks: Default::default(),
        }
    }

    /// Does the given election have a finalizer scheduled?
    pub async fn has_finalizer(&self, election: ElectionId) -> bool {
        self.tasks.lock().await.contains_key(&election)
    }

    /// Schedule a finalizer for every published and archived election.
    pub async fn schedule_elections(&self, db: &Database) -> Result<(), DbError> {
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
            self.schedule_election(unconfirmed_ballots, audited_ballots, &election)
                .await;
        }

        Ok(())
    }

    /// Schedule a finalizer for the given election.
    /// If one already exists, it will be rescheduled.
    pub async fn schedule_election(
        &self,
        unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
        audited_ballots: Coll<Ballot<Audited>>,
        election: &Election,
    ) {
        let finalizer = Self::finalizer(
            election.id,
            unconfirmed_ballots,
            audited_ballots,
            self.tasks.clone(),
        );
        // Schedule the finalizer and keep track of it.
        let mut tasks_locked = self.tasks.lock().await;
        if let Some(task) = tasks_locked.remove(&election.id) {
            let already_completed = task.cancel().await;
            if already_completed {
                // This should never happen, since a task can only complete by either:
                // * erroring, in which case it is replaced before returning.
                // * succeeding, in which case it is removed before returning.
                warn!(
                    "schedule_election: unexpected code path. This is not a bug in itself, \
but hints that assumptions made elsewhere might be incorrect"
                );
                return;
            }
        }
        let finalizer_task = ScheduledTask::new(finalizer, election.metadata.end_time);
        tasks_locked.insert(election.id, finalizer_task);
    }

    /// Immediately trigger the finalizer for the given election.
    /// If the finalizer was not previously scheduled (or already completed),
    /// this will have no effect.
    pub async fn finalize_election(&self, election_id: ElectionId) -> Result<(), Error> {
        let mut tasks_locked = self.tasks.lock().await;
        let task = tasks_locked.remove(&election_id);
        drop(tasks_locked); // Avoid deadlock, as the finalizer needs the lock too.
        match task {
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
    /// Since this is a recursive async function, we must use `BoxFuture` to
    /// avoid an infinitely-recursive state machine.
    fn finalizer(
        election_id: ElectionId,
        unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
        audited_ballots: Coll<Ballot<Audited>>,
        tasks: Arc<Mutex<TaskMap>>,
    ) -> BoxFuture<'static, Result<(), Error>> {
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

        async move {
            let result = finalize(election_id, unconfirmed_ballots.clone(), audited_ballots.clone()).await;
            match result {
                Ok(()) => {
                    tasks.lock().await.remove(&election_id);
                    trace!("Finalizer completed; removed self from list");
                }
                Err(ref e) => {
                    error!("Finalizer for election {election_id} failed, unconfirmed ballots might be leaked: {e}");
                    // Re-schedule the finalizer.
                    let retry = Self::finalizer(
                        election_id,
                        unconfirmed_ballots,
                        audited_ballots,
                        tasks.clone(),
                    );
                    const RETRY_INTERVAL_SECONDS: i64 = 300;
                    let retry_time = Utc::now() + Duration::seconds(RETRY_INTERVAL_SECONDS);
                    let mut tasks_locked = tasks.lock().await;
                    let finalizer_task = ScheduledTask::new(retry, retry_time);
                    tasks_locked.insert(election_id, finalizer_task);
                    warn!("Failed finalizer will be retried in {RETRY_INTERVAL_SECONDS} seconds");
                }
            }
            result
        }.boxed()
    }
}

impl Default for ElectionFinalizers {
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
        let election_finalizers = ElectionFinalizers::new();
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
        rocket = rocket.manage(election_finalizers);
        Ok(rocket)
    }
}

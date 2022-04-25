use std::collections::HashMap;

use mongodb::{bson::doc, Database};
use rocket::futures::TryStreamExt;
use rocket::http::Status;

use crate::error::{Error, Result};
use crate::model::{
    common::{
        ballot::{Audited, Unconfirmed},
        election::{ElectionId, ElectionState},
    },
    db::{ballot::Ballot, election::Election},
    mongodb::Coll,
};
use crate::scheduled_task::ScheduledTask;

/// Election finalizers: scheduled tasks for auditing unconfirmed ballots at the end of an election.
pub struct ElectionFinalizers(pub HashMap<ElectionId, ScheduledTask<Result<()>>>);

impl ElectionFinalizers {
    /// Create an empty set of election finalizers.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Schedule a finalizer for every published and archived election.
    pub async fn schedule_elections(&mut self, db: &Database) -> Result<()> {
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
    pub async fn finalize_election(&mut self, election_id: ElectionId) -> Result<()> {
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
    ) -> Result<()> {
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
        for ballot in ballots {
            let ballot = ballot.audit();
            let result = audited_ballots
                .replace_one(ballot.internal_id.as_doc(), &ballot, None)
                .await?;
            assert_eq!(result.modified_count, 1);
        }
        Ok(())
    }
}

impl Default for ElectionFinalizers {
    fn default() -> Self {
        Self::new()
    }
}

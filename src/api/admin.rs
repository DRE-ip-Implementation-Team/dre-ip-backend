use argon2::Config;
use chrono::Utc;
use mongodb::{bson::doc, Client};
use rand::Rng;
use rocket::{http::Status, serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    model::{
        api::auth::AuthToken,
        base::{ElectionSpec, ElectionState, NewAdmin},
        db::{Admin, CandidateTotals, ElectionNoSecrets, FinishedBallot, NewElection, Voter},
        mongodb::{Coll, Id},
    },
};

pub fn routes() -> Vec<Route> {
    routes![
        create_admin,
        delete_admin,
        create_election,
        modify_election,
        publish_election,
        archive_election,
        delete_election,
    ]
}

/// Raw admin credentials, received from a user. These are never stored directly,
/// since the password is in plaintext.
#[derive(Clone, Deserialize, Serialize)]
pub struct AdminCredentials {
    pub username: String,
    pub password: String,
}

impl From<AdminCredentials> for NewAdmin {
    /// Convert [`AdminCredentials`] to a new [`Admin`] by hashing the password.
    fn from(cred: AdminCredentials) -> Self {
        // 16 bytes is recommended for password hashing:
        //  https://en.wikipedia.org/wiki/Argon2
        // Also useful:
        //  https://www.twelve21.io/how-to-choose-the-right-parameters-for-argon2/
        let mut salt = [0_u8; 16];
        rand::thread_rng().fill(&mut salt);
        let password_hash = argon2::hash_encoded(
            cred.password.as_bytes(),
            &salt,
            &Config::default(), // TODO: see if a custom config is useful.
        )
        .unwrap(); // Safe because the default `Config` is valid.
        Self {
            username: cred.username,
            password_hash,
        }
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    impl AdminCredentials {
        pub fn example() -> Self {
            Self {
                username: "coordinator".into(),
                password: "coordinator".into(),
            }
        }

        pub fn example2() -> Self {
            Self {
                username: "coordinator2".into(),
                password: "coordinator2".into(),
            }
        }

        pub fn empty() -> Self {
            Self {
                username: "".into(),
                password: "".into(),
            }
        }
    }
}

#[post("/admins", data = "<new_admin>", format = "json")]
async fn create_admin(
    _token: AuthToken<Admin>,
    new_admin: Json<AdminCredentials>,
    admins: Coll<NewAdmin>,
) -> Result<()> {
    // Username uniqueness is enforced by the unique index on the username field.
    let admin: NewAdmin = new_admin.0.into();
    admins.insert_one(admin, None).await?;
    Ok(())
}

#[delete("/admins", data = "<username>", format = "json")]
async fn delete_admin(
    _token: AuthToken<Admin>,
    username: String,
    admins: Coll<Admin>,
) -> Result<()> {
    // Prevent deleting the last admin.
    let count = admins.count_documents(None, None).await?;
    if count == 1 {
        return Err(Error::Status(
            Status::UnprocessableEntity,
            "Cannot delete last admin!".to_string(),
        ));
    }

    let filter = doc! {
        "username": &username,
    };
    let result = admins.delete_one(filter, None).await?;
    if result.deleted_count == 0 {
        Err(Error::not_found(format!("Admin {}", username)))
    } else {
        Ok(())
    }
}

#[post("/elections", data = "<spec>", format = "json")]
async fn create_election(
    _token: AuthToken<Admin>,
    spec: Json<ElectionSpec>,
    new_elections: Coll<NewElection>,
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<ElectionNoSecrets>> {
    let election: NewElection = spec.0.into();
    let new_id: Id = new_elections
        .insert_one(&election, None)
        .await?
        .inserted_id
        .as_object_id()
        .unwrap() // Valid because the ID comes directly from the DB
        .into();
    let db_election = elections.find_one(new_id.as_doc(), None).await?.unwrap();
    Ok(Json(db_election.erase_secrets()))
}

#[put("/elections/<election_id>", data = "<spec>", format = "json")]
async fn modify_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    spec: Json<ElectionSpec>,
    new_elections: Coll<NewElection>,
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<ElectionNoSecrets>> {
    // Get the existing election.
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election {}", election_id)))?;

    // Check we are allowed to modify it.
    let now = Utc::now();
    if !(election.metadata.state == ElectionState::Draft
        || election.metadata.state == ElectionState::Published
            && election.metadata.start_time > now)
    {
        return Err(Error::Status(
            Status::BadRequest,
            format!("Cannot modify election {}", election_id),
        ));
    }

    // Replace with the new spec.
    let new_election: NewElection = spec.0.into();
    let result = new_elections
        .replace_one(election_id.as_doc(), &new_election, None)
        .await?;
    assert_eq!(result.modified_count, 1);

    let db_election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .unwrap();
    Ok(Json(db_election.erase_secrets()))
}

#[post("/elections/<election_id>/publish")]
async fn publish_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<ElectionNoSecrets>,
) -> Result<()> {
    // Update the state.
    let filter = doc! {
        "_id": election_id,
        "state": ElectionState::Draft,
    };
    let update = doc! {
        "$set": {
            "state": ElectionState::Published,
        }
    };
    let result = elections.update_one(filter, update, None).await?;
    if result.modified_count != 1 {
        return Err(Error::Status(
            Status::BadRequest,
            format!(
                "Election {} doesn't exist or isn't a draft; cannot publish.",
                election_id
            ),
        ));
    }

    Ok(())
}

#[post("/elections/<election_id>/archive")]
async fn archive_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<ElectionNoSecrets>,
) -> Result<()> {
    // Update the state.
    let filter = doc! {
        "_id": election_id,
        "$or": [{"state": ElectionState::Draft}, {"state": ElectionState::Published}],
    };
    let update = doc! {
        "$set": {
            "state": ElectionState::Archived,
        }
    };
    let result = elections.update_one(filter, update, None).await?;
    if result.modified_count != 1 {
        return Err(Error::Status(
            Status::BadRequest,
            format!(
                "Election {} doesn't exist or is already archived.",
                election_id
            ),
        ));
    }

    Ok(())
}

#[delete("/elections/<election_id>")]
async fn delete_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<ElectionNoSecrets>,
    ballots: Coll<FinishedBallot>,
    totals: Coll<CandidateTotals>,
    voters: Coll<Voter>,
    db_client: &State<Client>,
) -> Result<()> {
    // Get the election.
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election {}", election_id)))?;

    // Check that the election is in a deletable state.
    if !(election.metadata.state == ElectionState::Draft
        || election.metadata.state == ElectionState::Archived)
    {
        return Err(Error::Status(
            Status::BadRequest,
            format!("Cannot delete election {}", election_id),
        ));
    }

    // Atomically delete the election and all associated data.
    {
        let mut session = db_client.start_session(None).await?;
        session.start_transaction(None).await?;

        // Delete the election itself.
        let result = elections
            .delete_one_with_session(election_id.as_doc(), None, &mut session)
            .await?;
        assert_eq!(result.deleted_count, 1);

        // Delete all ballots and totals.
        let filter = doc! {
            "election_id": election_id,
        };
        ballots
            .delete_many_with_session(filter.clone(), None, &mut session)
            .await?;
        totals
            .delete_many_with_session(filter, None, &mut session)
            .await?;

        // Remove the election from all voters' allowed questions.
        let field_to_remove = format!("allowed_questions.{}", election_id);
        let update = doc! {
            "$unset": {
                &field_to_remove: "",
            }
        };
        voters
            .update_many_with_session(doc! {}, update, None, &mut session)
            .await?;

        session.commit_transaction().await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Duration;
    use mongodb::{bson::Document, Database};
    use rocket::{
        http::{ContentType, Status},
        local::asynchronous::{Client, LocalResponse},
        serde::json::serde_json,
    };

    use crate::model::{
        api::sms::Sms,
        base::{AllowedQuestions, ElectionMetadata, NewVoter, QuestionSpec},
        db::{Audited, Ballot, Confirmed, NewCandidateTotals, Unconfirmed},
        mongodb::MongoCollection,
    };
    use crate::Config;

    use super::*;

    #[backend_test(admin)]
    async fn create_delete_admin(client: Client, db: Database) {
        // Create admin
        let response = client
            .post(uri!(create_admin))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&AdminCredentials::example2()).unwrap())
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());

        // Ensure the admin has been inserted
        let admins = Coll::<Admin>::from_db(&db);
        let with_username = doc! { "username": &NewAdmin::example2().username };
        let inserted_admin = admins
            .find_one(with_username.clone(), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(NewAdmin::example2().username, inserted_admin.username);

        // Delete the admin.
        let count = admins.count_documents(None, None).await.unwrap();
        assert_eq!(count, 2);
        let response = client
            .delete(uri!(delete_admin))
            .header(ContentType::JSON)
            .body(AdminCredentials::example2().username)
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());

        // Ensure the admin has been deleted.
        let count = admins.count_documents(None, None).await.unwrap();
        assert_eq!(count, 1);
        let admin = admins.find_one(None, None).await.unwrap().unwrap();
        assert_eq!(admin.username, AdminCredentials::example().username);
    }

    #[backend_test(admin)]
    async fn create_election(client: Client, db: Database) {
        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ElectionSpec::current_example()).unwrap())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        let elections = Coll::<ElectionMetadata>::from_db(&db);
        let with_name = doc! { "name": &ElectionSpec::current_example().name };
        let inserted_election = elections
            .find_one(with_name.clone(), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            ElectionMetadata::from(ElectionSpec::current_example()),
            inserted_election
        );
    }

    #[backend_test(admin)]
    async fn publish_archive(client: Client, db: Database) {
        // Try to publish/archive an election that doesn't exist.
        publish_expect_status(&client, Id::new(), Status::BadRequest).await;
        archive_expect_status(&client, Id::new(), Status::BadRequest).await;

        // Create an election.
        let spec = ElectionSpec::current_example();
        let election = create_election_for_spec(&client, &spec).await;

        // Archive it.
        archive(&client, election.id).await;
        let archived = get_election_by_id(&db, election.id).await;
        assert_eq!(archived.metadata.state, ElectionState::Archived);

        // Check we can't publish it or archive it again.
        publish_expect_status(&client, election.id, Status::BadRequest).await;
        archive_expect_status(&client, election.id, Status::BadRequest).await;

        // Create a new election.
        let election = create_election_for_spec(&client, &spec).await;

        // Publish it.
        publish(&client, election.id).await;
        let published = get_election_by_id(&db, election.id).await;
        assert_eq!(published.metadata.state, ElectionState::Published);

        // Check we can't publish it again.
        publish_expect_status(&client, election.id, Status::BadRequest).await;

        // Archive it.
        archive(&client, election.id).await;
        let archived = get_election_by_id(&db, election.id).await;
        assert_eq!(archived.metadata.state, ElectionState::Archived);
    }

    #[backend_test(admin)]
    async fn modify_election(client: Client, db: Database) {
        // Try to modify an election that doesn't exist.
        modify_expect_status(
            &client,
            Id::new(),
            &ElectionSpec::current_example(),
            Status::NotFound,
        )
        .await;

        // Create an election.
        let mut spec = ElectionSpec::future_example();
        let election = create_election_for_spec(&client, &spec).await;

        // Modify it.
        spec.name = "New Name".to_string();
        let modified = modify_election_with_spec(&client, election.id, &spec).await;
        assert_eq!(modified, get_election_by_id(&db, election.id).await);
        assert_eq!(modified.metadata.name, spec.name);
        assert_eq!(modified.metadata.state, election.metadata.state);
        assert_eq!(modified.metadata.start_time, election.metadata.start_time);
        assert_eq!(modified.metadata.end_time, election.metadata.end_time);
        assert_eq!(modified.electorates, election.electorates);

        // Publish it.
        publish(&client, election.id).await;

        // Modify it again.
        spec.start_time = Utc::now();
        let modified = modify_election_with_spec(&client, election.id, &spec).await;
        assert_eq!(modified, get_election_by_id(&db, election.id).await);
        assert_eq!(modified.metadata.name, spec.name);
        assert_eq!(modified.metadata.state, ElectionState::Draft);
        assert!(modified.metadata.start_time - spec.start_time < Duration::seconds(1));
        assert_eq!(modified.metadata.end_time, election.metadata.end_time);
        assert_eq!(modified.electorates, election.electorates);

        // Re-publish.
        publish(&client, election.id).await;

        // Ensure we can't modify due to the new start date.
        modify_expect_status(
            &client,
            election.id,
            &ElectionSpec::current_example(),
            Status::BadRequest,
        )
        .await;

        // Archive it.
        archive(&client, election.id).await;

        // Ensure we can't modify an archived election.
        modify_expect_status(
            &client,
            election.id,
            &ElectionSpec::current_example(),
            Status::BadRequest,
        )
        .await;

        // Ensure we can't modify an election that went straight from draft to archived
        // while still being before the start time.
        let election = create_election_for_spec(&client, &ElectionSpec::future_example()).await;
        archive(&client, election.id).await;
        modify_expect_status(
            &client,
            election.id,
            &ElectionSpec::current_example(),
            Status::BadRequest,
        )
        .await;
    }

    #[backend_test(admin)]
    async fn delete_election(client: Client, db: Database) {
        // Try to delete an election that doesn't exist.
        delete_expect_status(&client, Id::new(), Status::NotFound).await;

        // Create an election.
        let spec = ElectionSpec::current_example();
        let election = create_election_for_spec(&client, &spec).await;

        // Delete it.
        delete(&client, election.id).await;
        assert_no_matches::<ElectionNoSecrets>(&db, election.id.as_doc()).await;

        // Create a new election.
        let election = create_election_for_spec(&client, &spec).await;

        // Publish it.
        publish(&client, election.id).await;

        // Check it can't be deleted.
        delete_expect_status(&client, election.id, Status::BadRequest).await;
        get_election_by_id(&db, election.id).await;

        // Archive it.
        archive(&client, election.id).await;

        // Delete it.
        delete(&client, election.id).await;
        assert_no_matches::<ElectionNoSecrets>(&db, election.id.as_doc()).await;

        // Create an active election.
        let election = create_election_for_spec(&client, &spec).await;
        publish(&client, election.id).await;

        // Cast, audit, and confirm various votes.
        insert_ballots(&db, election.id).await;
        let voters = insert_allowed_questions(&client, &db, election.id).await;

        // Delete it.
        archive(&client, election.id).await;
        delete(&client, election.id).await;
        let filter = doc! {
            "election_id": *election.id,
        };
        assert_no_matches::<ElectionNoSecrets>(&db, election.id.as_doc()).await;
        assert_no_matches::<FinishedBallot>(&db, filter.clone()).await;
        assert_no_matches::<Ballot<Unconfirmed>>(&db, filter.clone()).await;
        assert_no_matches::<CandidateTotals>(&db, filter).await;
        let field_name = format!("allowed_questions.{}", election.id);
        let filter = doc! {
            &field_name: {
                "$exists": true,
            }
        };
        assert_no_matches::<Voter>(&db, filter).await;
        // Check we didn't accidentally remove allowed questions from a different election.
        let other_voter = Coll::<Voter>::from_db(&db)
            .find_one(voters[3].as_doc(), None)
            .await
            .unwrap()
            .unwrap();
        assert!(!other_voter.allowed_questions.is_empty());
    }

    async fn get_election_by_id(db: &Database, id: Id) -> ElectionNoSecrets {
        Coll::<ElectionNoSecrets>::from_db(db)
            .find_one(id.as_doc(), None)
            .await
            .unwrap()
            .unwrap()
    }

    async fn assert_no_matches<T: MongoCollection>(db: &Database, filter: Document) {
        let matches = Coll::<T>::from_db(db)
            .count_documents(filter, None)
            .await
            .unwrap();
        assert_eq!(matches, 0);
    }

    async fn create_election_for_spec(client: &Client, spec: &ElectionSpec) -> ElectionNoSecrets {
        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(serde_json::to_string(spec).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        serde_json::from_str(&response.into_string().await.unwrap()).unwrap()
    }

    async fn modify_election_with_spec(
        client: &Client,
        id: Id,
        spec: &ElectionSpec,
    ) -> ElectionNoSecrets {
        let response = modify_expect_status(client, id, spec, Status::Ok).await;
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap()
    }

    async fn modify_expect_status<'c>(
        client: &'c Client,
        id: Id,
        spec: &ElectionSpec,
        status: Status,
    ) -> LocalResponse<'c> {
        let response = client
            .put(uri!(modify_election(id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(spec).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), status);
        response
    }

    async fn publish(client: &Client, id: Id) {
        publish_expect_status(client, id, Status::Ok).await
    }

    async fn publish_expect_status(client: &Client, id: Id, status: Status) {
        let response = client.post(uri!(publish_election(id))).dispatch().await;
        assert_eq!(response.status(), status);
    }

    async fn archive(client: &Client, id: Id) {
        archive_expect_status(client, id, Status::Ok).await
    }

    async fn archive_expect_status(client: &Client, id: Id, status: Status) {
        let response = client.post(uri!(archive_election(id))).dispatch().await;
        assert_eq!(response.status(), status);
    }

    async fn delete(client: &Client, id: Id) {
        delete_expect_status(client, id, Status::Ok).await
    }

    async fn delete_expect_status(client: &Client, id: Id, status: Status) {
        let response = client.delete(uri!(delete_election(id))).dispatch().await;
        assert_eq!(response.status(), status);
    }

    async fn insert_ballots(db: &Database, election_id: Id) {
        let election = get_election_by_id(db, election_id).await;
        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap();
        let q1c1 = q1.candidates.get(0).unwrap();
        let q1c2 = q1.candidates.get(1).unwrap();
        let q2 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example2().description)
            .unwrap();
        let q2c1 = q2.candidates.get(0).unwrap();
        let q2c2 = q2.candidates.get(1).unwrap();
        let mut rng = rand::thread_rng();

        let mut candidate_totals = Vec::new();
        for candidate in q1.candidates.iter() {
            candidate_totals.push(NewCandidateTotals::new(
                election.id,
                q1.id,
                candidate.clone(),
            ));
        }
        for candidate in q2.candidates.iter() {
            candidate_totals.push(NewCandidateTotals::new(
                election.id,
                q2.id,
                candidate.clone(),
            ));
        }
        // This relies on no duplicate candidate names between questions, which is true for the examples.
        let mut totals_map = candidate_totals
            .iter_mut()
            .map(|t| (t.candidate_name.clone(), &mut t.crypto))
            .collect::<HashMap<_, _>>();

        macro_rules! ballot {
            ($q:ident, $yes:ident, $no:ident) => {
                Ballot::new($q.id, $yes.clone(), vec![$no.clone()], &election, &mut rng).unwrap()
            };
        }

        // Create confirmed ballots.
        let confirmed = vec![
            // q1: 3 votes for candidate 1, 2 votes for candidate 2
            ballot!(q1, q1c1, q1c2).confirm(&mut totals_map),
            ballot!(q1, q1c1, q1c2).confirm(&mut totals_map),
            ballot!(q1, q1c1, q1c2).confirm(&mut totals_map),
            ballot!(q1, q1c2, q1c1).confirm(&mut totals_map),
            ballot!(q1, q1c2, q1c1).confirm(&mut totals_map),
            // q2: 3 votes for candidate 2
            ballot!(q2, q2c2, q2c1).confirm(&mut totals_map),
            ballot!(q2, q2c2, q2c1).confirm(&mut totals_map),
            ballot!(q2, q2c2, q2c1).confirm(&mut totals_map),
        ];

        // Create audited ballots.
        let audited = vec![
            // q1: 1 vote for each
            ballot!(q1, q1c1, q1c2).audit(),
            ballot!(q1, q1c2, q1c1).audit(),
            // q2: 3 votes for candidate 2, 1 vote for candidate 1
            ballot!(q2, q2c2, q2c1).audit(),
            ballot!(q2, q2c2, q2c1).audit(),
            ballot!(q2, q2c2, q2c1).audit(),
            ballot!(q2, q2c1, q2c2).audit(),
        ];

        // Create unconfirmed ballots.
        let unconfirmed = vec![
            // q1: 1 vote for each
            ballot!(q1, q1c1, q1c2),
            ballot!(q1, q1c2, q1c1),
            // q2: 1 vote for candidate 1
            ballot!(q2, q2c1, q2c2),
        ];

        // Insert ballots.
        Coll::<Ballot<Confirmed>>::from_db(db)
            .insert_many(confirmed, None)
            .await
            .unwrap();
        Coll::<Ballot<Audited>>::from_db(db)
            .insert_many(audited, None)
            .await
            .unwrap();
        Coll::<Ballot<Unconfirmed>>::from_db(db)
            .insert_many(unconfirmed, None)
            .await
            .unwrap();
        // Insert candidate totals.
        Coll::<NewCandidateTotals>::from_db(db)
            .insert_many(candidate_totals, None)
            .await
            .unwrap();
    }

    async fn insert_allowed_questions(client: &Client, db: &Database, election_id: Id) -> Vec<Id> {
        let config = client.rocket().state::<Config>().unwrap();
        let election = get_election_by_id(&db, election_id).await;
        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap()
            .id;
        let q2 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example2().description)
            .unwrap()
            .id;
        let q3 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example3().description)
            .unwrap()
            .id;

        // First voter has voted on everything.
        let allowed_questions = HashMap::from_iter(vec![(
            election_id,
            AllowedQuestions {
                confirmed: HashMap::from_iter(vec![(q1, true), (q2, true), (q3, true)]),
            },
        )]);
        let voter1 = NewVoter {
            sms_hmac: "+441234567890".parse::<Sms>().unwrap().into_hmac(config),
            allowed_questions,
        };

        // Second voter has voted on some.
        let allowed_questions = HashMap::from_iter(vec![(
            election_id,
            AllowedQuestions {
                confirmed: HashMap::from_iter(vec![(q1, true), (q3, false)]),
            },
        )]);
        let voter2 = NewVoter {
            sms_hmac: "+440987654321".parse::<Sms>().unwrap().into_hmac(config),
            allowed_questions,
        };

        // Third voter is not allowed to vote on any.
        let allowed_questions = HashMap::from_iter(vec![(
            election_id,
            AllowedQuestions {
                confirmed: HashMap::new(),
            },
        )]);
        let voter3 = NewVoter {
            sms_hmac: "+440123443210".parse::<Sms>().unwrap().into_hmac(config),
            allowed_questions,
        };

        // Fourth voter never even joined.
        let allowed_questions = HashMap::from_iter(vec![(
            Id::new(),
            AllowedQuestions {
                confirmed: HashMap::new(),
            },
        )]);
        let voter4 = NewVoter {
            sms_hmac: "+444321001234".parse::<Sms>().unwrap().into_hmac(config),
            allowed_questions,
        };

        let result = Coll::<NewVoter>::from_db(db)
            .insert_many(vec![voter1, voter2, voter3, voter4], None)
            .await
            .unwrap();
        let mut resulting_ids = Vec::with_capacity(4);
        for i in 0..4 {
            resulting_ids.push(result.inserted_ids.get(&i).unwrap());
        }
        resulting_ids
            .into_iter()
            .map(|v| v.as_object_id().unwrap().into())
            .collect()
    }
}

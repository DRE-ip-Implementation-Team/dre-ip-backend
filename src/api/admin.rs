use chrono::Utc;
use mongodb::{bson::doc, Client};
use rocket::{futures::TryStreamExt, http::Status, serde::json::Json, Route, State};

use crate::{
    error::{Error, Result},
    model::{
        api::{
            admin::AdminCredentials,
            auth::AuthToken,
            election::{ElectionDescription, ElectionSpec},
        },
        common::{
            ballot::{Audited, Unconfirmed},
            election::ElectionState,
        },
        db::{
            admin::{Admin, NewAdmin},
            ballot::{Ballot, FinishedBallot},
            candidate_totals::CandidateTotals,
            election::{Election, NewElection},
            voter::Voter,
        },
        mongodb::{Coll, Counter, Id},
    },
    ElectionFinalizers,
};

pub fn routes() -> Vec<Route> {
    routes![
        get_admins,
        create_admin,
        delete_admin,
        create_election,
        modify_election,
        publish_election,
        archive_election,
        delete_election,
    ]
}

#[get("/admins")]
async fn get_admins(_token: AuthToken<Admin>, admins: Coll<Admin>) -> Result<Json<Vec<String>>> {
    let admin_list: Vec<Admin> = admins.find(None, None).await?.try_collect().await?;
    let admin_names = admin_list
        .into_iter()
        .map(|admin| admin.admin.username)
        .collect();
    Ok(Json(admin_names))
}

#[post("/admins", data = "<new_admin>", format = "json")]
async fn create_admin(
    _token: AuthToken<Admin>,
    new_admin: Json<AdminCredentials>,
    admins: Coll<NewAdmin>,
) -> Result<()> {
    // Check username uniqueness.
    let filter = doc! {
        "username": &new_admin.username,
    };
    let existing = admins.find_one(filter, None).await?;
    if existing.is_some() {
        return Err(Error::Status(
            Status::BadRequest,
            format!("Admin username already in use: {}", new_admin.username),
        ));
    }

    // Create and insert the admin.
    let admin: NewAdmin = new_admin
        .0
        .try_into()
        .map_err(|_| Error::Status(Status::BadRequest, "Illegal admin credentials".to_string()))?;
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
    elections: Coll<Election>,
    counters: Coll<Counter>,
    db_client: &State<Client>,
) -> Result<Json<ElectionDescription>> {
    let election = {
        let mut session = db_client.start_session(None).await?;
        session.start_transaction(None).await?;

        // Create and insert the election.
        let election: NewElection = spec.0.into();
        let new_id: Id = new_elections
            .insert_one_with_session(&election, None, &mut session)
            .await?
            .inserted_id
            .as_object_id()
            .unwrap() // Valid because the ID comes directly from the DB
            .into();

        // Retrieve the full election information including ID.
        let election = elections
            .find_one_with_session(new_id.as_doc(), None, &mut session)
            .await?
            .unwrap();

        // Create and insert a counter for each question.
        let new_counters = election
            .questions
            .keys()
            .map(|question_id| Counter::new(*question_id, 1))
            .collect::<Vec<_>>();
        counters
            .insert_many_with_session(&new_counters, None, &mut session)
            .await?;

        session.commit_transaction().await?;
        election
    };

    Ok(Json(election.into()))
}

#[put("/elections/<election_id>", data = "<spec>", format = "json")]
async fn modify_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    spec: Json<ElectionSpec>,
    new_elections: Coll<NewElection>,
    elections: Coll<Election>,
) -> Result<Json<ElectionDescription>> {
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
    Ok(Json(db_election.into()))
}

#[post("/elections/<election_id>/publish")]
async fn publish_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<Election>,
    unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
    audited_ballots: Coll<Ballot<Audited>>,
    election_finalizers: &State<ElectionFinalizers>,
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

    // Schedule the election finalizer.
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .unwrap(); // Presence already checked.
    election_finalizers.lock().await.schedule_election(
        unconfirmed_ballots,
        audited_ballots,
        &election,
    );

    Ok(())
}

#[post("/elections/<election_id>/archive")]
async fn archive_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<Election>,
    election_finalizers: &State<ElectionFinalizers>,
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

    // Run the election finalizer.
    election_finalizers
        .lock()
        .await
        .finalize_election(election_id)
        .await?;

    Ok(())
}

#[delete("/elections/<election_id>")]
#[allow(clippy::too_many_arguments)]
async fn delete_election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<Election>,
    ballots: Coll<FinishedBallot>,
    totals: Coll<CandidateTotals>,
    voters: Coll<Voter>,
    counters: Coll<Counter>,
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

        // Delete the counters.
        for question_id in election.questions.keys() {
            let result = counters
                .delete_one_with_session(question_id.as_doc(), None, &mut session)
                .await?;
            assert_eq!(result.deleted_count, 1);
        }

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
        tokio,
    };

    use crate::model::{
        api::{
            election::{ElectionSpec, QuestionSpec},
            sms::Sms,
        },
        common::{
            allowed_questions::AllowedQuestions,
            ballot::{Audited, Confirmed, Unconfirmed},
        },
        db::{
            admin::DEFAULT_ADMIN_USERNAME,
            ballot::{Ballot, BallotCore},
            candidate_totals::NewCandidateTotals,
            election::ElectionMetadata,
            voter::NewVoter,
        },
        mongodb::MongoCollection,
    };
    use crate::Config;

    use super::*;

    #[backend_test(admin)]
    async fn create_delete_admin(client: Client, db: Database) {
        // Create admin
        create_admin(&client, &AdminCredentials::example2()).await;

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
        assert_eq!(count, 3); // Default admin, test admin, new admin.
        let response = client
            .delete(uri!(delete_admin))
            .header(ContentType::JSON)
            .body(AdminCredentials::example2().username)
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());

        // Ensure the admin has been deleted.
        let count = admins.count_documents(None, None).await.unwrap();
        assert_eq!(count, 2);
        let expected = vec![
            DEFAULT_ADMIN_USERNAME.to_string(),
            AdminCredentials::example1().username,
        ];
        let remaining_admins: Vec<String> = admins
            .find(None, None)
            .await
            .unwrap()
            .map_ok(|a| a.admin.username)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(expected, remaining_admins);
    }

    #[backend_test(admin)]
    async fn bad_create_admin(client: Client, db: Database) {
        // Try empty username.
        let credentials = AdminCredentials {
            username: "".to_string(),
            password: "foo".to_string(),
        };
        create_admin_expect_status(&client, &credentials, Status::BadRequest).await;

        // Try empty password.
        let credentials = AdminCredentials {
            username: "foo".to_string(),
            password: "".to_string(),
        };
        create_admin_expect_status(&client, &credentials, Status::BadRequest).await;

        // Try empty both.
        create_admin_expect_status(&client, &AdminCredentials::empty(), Status::BadRequest).await;

        // Try duplicate username.
        create_admin_expect_status(&client, &AdminCredentials::example1(), Status::BadRequest)
            .await;

        // Ensure no admins were created.
        let num_admins = count_matches::<Admin>(&db, doc! {}).await;
        assert_eq!(num_admins, 2); // Default admin and test admin.
    }

    #[backend_test(admin)]
    async fn get_admins(client: Client) {
        // Create some admins.
        create_admin(&client, &AdminCredentials::example2()).await;
        create_admin(&client, &AdminCredentials::example3()).await;

        // Check that all admins are listed.
        let response = client.get(uri!(get_admins)).dispatch().await;
        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let admins: Vec<String> =
            serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
        let expected = vec![
            DEFAULT_ADMIN_USERNAME.to_string(),
            AdminCredentials::example1().username,
            AdminCredentials::example2().username,
            AdminCredentials::example3().username,
        ];
        assert_eq!(admins, expected);
    }

    #[backend_test(admin)]
    async fn create_election(client: Client, db: Database) {
        // Create an election.
        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ElectionSpec::current_example()).unwrap())
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());
        let raw_response = response.into_string().await.unwrap();
        let response_election: ElectionDescription = serde_json::from_str(&raw_response).unwrap();

        // Ensure it is present in the DB.
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

        // Ensure the counters were created.
        for question_id in response_election.questions.keys() {
            let counter = Coll::<Counter>::from_db(&db)
                .find_one(question_id.as_doc(), None)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(counter.next, 1);
        }
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
        archive(&client, *election.id).await;
        let archived = get_election_by_id(&db, *election.id).await;
        assert_eq!(archived.metadata.state, ElectionState::Archived);

        // Check we can't publish it or archive it again.
        publish_expect_status(&client, *election.id, Status::BadRequest).await;
        archive_expect_status(&client, *election.id, Status::BadRequest).await;

        // Create a new election.
        let election = create_election_for_spec(&client, &spec).await;

        // Publish it.
        publish(&client, *election.id).await;
        let published = get_election_by_id(&db, *election.id).await;
        assert_eq!(published.metadata.state, ElectionState::Published);

        // Check we can't publish it again.
        publish_expect_status(&client, *election.id, Status::BadRequest).await;

        // Archive it.
        archive(&client, *election.id).await;
        let archived = get_election_by_id(&db, *election.id).await;
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
        let modified = modify_election_with_spec(&client, *election.id, &spec).await;
        assert_eq!(modified, get_election_by_id(&db, *election.id).await.into());
        assert_eq!(modified.name, spec.name);
        assert_eq!(modified.state, election.state);
        assert_eq!(modified.start_time, election.start_time);
        assert_eq!(modified.end_time, election.end_time);
        assert_eq!(modified.electorates, election.electorates);

        // Publish it.
        publish(&client, *election.id).await;

        // Modify it again.
        spec.start_time = Utc::now();
        let modified = modify_election_with_spec(&client, *election.id, &spec).await;
        assert_eq!(modified, get_election_by_id(&db, *election.id).await.into());
        assert_eq!(modified.name, spec.name);
        assert_eq!(modified.state, ElectionState::Draft);
        assert!(modified.start_time - spec.start_time < Duration::seconds(1));
        assert_eq!(modified.end_time, election.end_time);
        assert_eq!(modified.electorates, election.electorates);

        // Re-publish.
        publish(&client, *election.id).await;

        // Ensure we can't modify due to the new start date.
        modify_expect_status(
            &client,
            *election.id,
            &ElectionSpec::current_example(),
            Status::BadRequest,
        )
        .await;

        // Archive it.
        archive(&client, *election.id).await;

        // Ensure we can't modify an archived election.
        modify_expect_status(
            &client,
            *election.id,
            &ElectionSpec::current_example(),
            Status::BadRequest,
        )
        .await;

        // Ensure we can't modify an election that went straight from draft to archived
        // while still being before the start time.
        let election = create_election_for_spec(&client, &ElectionSpec::future_example()).await;
        archive(&client, *election.id).await;
        modify_expect_status(
            &client,
            *election.id,
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
        delete(&client, *election.id).await;
        assert_no_matches::<Election>(&db, election.id.as_doc()).await;

        // Create a new election.
        let election = create_election_for_spec(&client, &spec).await;

        // Publish it.
        publish(&client, *election.id).await;

        // Check it can't be deleted.
        delete_expect_status(&client, *election.id, Status::BadRequest).await;
        get_election_by_id(&db, *election.id).await;

        // Archive it.
        archive(&client, *election.id).await;

        // Delete it.
        delete(&client, *election.id).await;
        assert_no_matches::<Election>(&db, election.id.as_doc()).await;

        // Create an active election.
        let election = create_election_for_spec(&client, &spec).await;
        publish(&client, *election.id).await;

        // Cast, audit, and confirm various votes.
        insert_ballots(&db, *election.id).await;
        let voters = insert_allowed_questions(&client, &db, *election.id).await;

        // Delete it.
        archive(&client, *election.id).await;
        delete(&client, *election.id).await;
        let filter = doc! {
            "election_id": *election.id,
        };
        assert_no_matches::<Election>(&db, election.id.as_doc()).await;
        assert_no_matches::<Counter>(&db, election.id.as_doc()).await;
        // Since the filter doesn't specify a state, using the FinishedBallot
        // collection here actually includes unconfirmed ballots as well.
        assert_no_matches::<FinishedBallot>(&db, filter.clone()).await;
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

    #[backend_test(admin)]
    async fn finalize_on_archive(client: Client, db: Database) {
        // Create an election, publish it, and add votes.
        let spec = ElectionSpec::current_example();
        let election = create_election_for_spec(&client, &spec).await;
        publish(&client, *election.id).await;
        insert_ballots(&db, *election.id).await;

        // Check there are unconfirmed ballots.
        let unconfirmed_filter = doc! {
            "election_id": *election.id,
            "state": Unconfirmed,
        };
        let unconfirmed =
            count_matches::<Ballot<Unconfirmed>>(&db, unconfirmed_filter.clone()).await;
        assert_ne!(unconfirmed, 0);
        let audited_filter = doc! {
            "election_id": *election.id,
            "state": Audited,
        };
        let audited = count_matches::<Ballot<Audited>>(&db, audited_filter.clone()).await;

        // Check a finalizer has been scheduled.
        let finalizers = client.rocket().state::<ElectionFinalizers>().unwrap();
        assert!(finalizers.lock().await.0.contains_key(&election.id));

        // Archive the election.
        archive(&client, *election.id).await;
        // Check the unconfirmed ballots have been audited, i.e. the finalizer was triggered.
        assert_no_matches::<Ballot<Unconfirmed>>(&db, unconfirmed_filter.clone()).await;
        let final_audited = count_matches::<Ballot<Audited>>(&db, audited_filter).await;
        assert_eq!(final_audited, audited + unconfirmed);
    }

    #[backend_test(admin)]
    async fn finalize_on_end_time(client: Client, db: Database) {
        // Create an election in the past and add some votes.
        let spec = ElectionSpec::past_example();
        let election = create_election_for_spec(&client, &spec).await;
        insert_ballots(&db, *election.id).await;

        // Check there are unconfirmed ballots.
        let unconfirmed_filter = doc! {
            "election_id": *election.id,
            "state": Unconfirmed,
        };
        let unconfirmed =
            count_matches::<Ballot<Unconfirmed>>(&db, unconfirmed_filter.clone()).await;
        assert_ne!(unconfirmed, 0);
        let audited_filter = doc! {
            "election_id": *election.id,
            "state": Audited,
        };
        let audited = count_matches::<Ballot<Audited>>(&db, audited_filter.clone()).await;

        // Publish it, causing a finalizer to be scheduled that should immediately trigger.
        publish(&client, *election.id).await;
        // (hopefully not flaky) sleep to make sure the finalizers have gone through.
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Check the unconfirmed ballots have been audited, i.e. the finalizer was triggered.
        assert_no_matches::<Ballot<Unconfirmed>>(&db, unconfirmed_filter.clone()).await;
        let final_audited = count_matches::<Ballot<Audited>>(&db, audited_filter).await;
        assert_eq!(final_audited, audited + unconfirmed);
    }

    async fn get_election_by_id(db: &Database, id: Id) -> Election {
        Coll::<Election>::from_db(db)
            .find_one(id.as_doc(), None)
            .await
            .unwrap()
            .unwrap()
    }

    async fn count_matches<T: MongoCollection>(db: &Database, filter: Document) -> u64 {
        Coll::<T>::from_db(db)
            .count_documents(filter, None)
            .await
            .unwrap()
    }

    async fn assert_no_matches<T: MongoCollection>(db: &Database, filter: Document) {
        let matches = count_matches::<T>(db, filter).await;
        assert_eq!(matches, 0);
    }

    async fn create_election_for_spec(client: &Client, spec: &ElectionSpec) -> ElectionDescription {
        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(serde_json::to_string(spec).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        serde_json::from_str(&response.into_string().await.unwrap()).unwrap()
    }

    async fn create_admin(client: &Client, spec: &AdminCredentials) {
        create_admin_expect_status(client, spec, Status::Ok).await
    }

    async fn create_admin_expect_status(client: &Client, spec: &AdminCredentials, status: Status) {
        let response = client
            .post(uri!(create_admin))
            .header(ContentType::JSON)
            .body(serde_json::to_string(spec).unwrap())
            .dispatch()
            .await;
        assert_eq!(status, response.status());
    }

    async fn modify_election_with_spec(
        client: &Client,
        id: Id,
        spec: &ElectionSpec,
    ) -> ElectionDescription {
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

    // Clippy doesn't like the ballot!() macros inside the vec![] macro, since
    // the order of resolving the ballot ID increments depends on the order of
    // evaluating the vector elements. It's fine though - the order doesn't matter.
    #[allow(clippy::eval_order_dependence)]
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

        let mut next_ballot_id = 1;
        macro_rules! ballot {
            ($q:ident, $yes:ident, $no:ident) => {{
                next_ballot_id += 1;
                BallotCore::new(
                    next_ballot_id,
                    $q.id,
                    $yes.clone(),
                    vec![$no.clone()],
                    &election,
                    &mut rng,
                )
                .unwrap()
            }};
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
        Coll::<BallotCore<Confirmed>>::from_db(db)
            .insert_many(confirmed, None)
            .await
            .unwrap();
        Coll::<BallotCore<Audited>>::from_db(db)
            .insert_many(audited, None)
            .await
            .unwrap();
        Coll::<BallotCore<Unconfirmed>>::from_db(db)
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
        let election = get_election_by_id(db, election_id).await;
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

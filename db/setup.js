db.dropDatabase();

// Voter collection
db.createCollection("voters");
voters = db.getCollection("voters");
voters.createIndex({sms_hmac: 1}, {unique: true});

// Admin collection
db.createCollection("admins");
admins = db.getCollection("admins");
admins.createIndex({username: 1}, {unique: true});

// Election collection
db.createCollection("elections");
elections = db.getCollection("elections");

// Ballot collection
db.createCollection("ballots");
ballots = db.getCollection("ballots");
ballots.createIndex({election_id: 1, question_id: 1})
ballots.createIndex({creation_time: -1}, {
	expireAfterSeconds: 3600,
	partialFilterExpression: {state: "Unconfirmed"}
})

// Candidate totals collection
db.createCollection("candidate_totals");
candidate_totals = db.getCollection("candidate_totals");
candidate_totals.createIndex({election_id: 1, question_id: 1, candidate_name: 1})

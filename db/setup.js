<<<<<<< HEAD:setup.js
conn = new Mongo();
db = conn.getDB("dreip");
db.dropDatabase();

// Voter collection
db.createCollection("voters");
voters = db.getCollection("voters");
voters.createIndex({sms: 1}, {unique: true});

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
ballots.createIndex({election_id: 1, question_no: 1})
||||||| f80f4fb:setup.js
conn = new Mongo();
db = conn.getDB("dreip");
db.dropDatabase();
db.createCollection("voters");
voters = db.getCollection("voters");
voters.createIndex({sms: 1}, {unique: true});
db.createCollection("admins");
admins = db.getCollection("admins");
admins.createIndex({username: 1}, {unique: true});
=======
db.dropDatabase();
db.createCollection("voters");
voters = db.getCollection("voters");
voters.createIndex({sms: 1}, {unique: true});
db.createCollection("admins");
admins = db.getCollection("admins");
admins.createIndex({username: 1}, {unique: true});
>>>>>>> 0ac1c47783170cead0020847aafa0a1a0b74441b:db/setup.js

db.dropDatabase();
db.createCollection("voters");
voters = db.getCollection("voters");
voters.createIndex({sms: 1}, {unique: true});
db.createCollection("admins");
admins = db.getCollection("admins");
admins.createIndex({username: 1}, {unique: true});

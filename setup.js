conn = new Mongo();
db = conn.getDB("dreip");
db.dropDatabase();
db.createCollection("voters");
voters = db.getCollection("voters");
voters.createIndex({sms: 1}, {unique: true});
db.createCollection("admins");
admins = db.getCollection("users");
admins.createIndex({username: 1}, {unique: true});

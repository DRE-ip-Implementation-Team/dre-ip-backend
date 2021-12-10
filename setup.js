conn = new Mongo();
db = conn.getDB("dreip");
db.dropDatabase();
db.createCollection("users");
users = db.getCollection("users");
users.createIndex({sms: 1}, {unique: true});

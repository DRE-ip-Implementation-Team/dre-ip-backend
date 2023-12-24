use flags;
db.createCollection("flags");
db.flags.insertOne({"init_complete": true});

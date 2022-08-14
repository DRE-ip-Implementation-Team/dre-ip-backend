use dreip;
// Create an admin account.
db.admins.insertOne({
  username: "admin",
  // Password is `admin`
  password_hash: "$argon2i$v=19$m=4096,t=3,p=1$WDluTWJ4ZHUyOXRwaUJtcg$PRV0VWV3M4uNMiYZ/85QqB8rkVLHuWoyCq+CkRb9Opw"
});

// Create the counters.
db.counters.insertMany([
  {
    "_id": "eid",
    "next": 1
  }
]);

mod bson;
mod collection;

pub use bson::{serde_string_map, Id};
pub use collection::{Coll, MongoCollection};

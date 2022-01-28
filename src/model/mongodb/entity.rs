use super::bson::Id;

pub trait DbEntity {
    fn id(&self) -> Id;
}

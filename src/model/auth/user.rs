use std::fmt::Display;

use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::model::{
    admin::{db::DbAdmin, Admin},
    mongodb::entity::DbEntity,
    voter::{db::DbVoter, Voter},
};

pub trait User {
    type DbUser: DbEntity;

    fn rights() -> Rights;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Rights {
    Voter = 0,
    Admin = 1,
}

impl Display for Rights {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                Self::Voter => "voter",
                Self::Admin => "admin",
            }
        )
    }
}

impl User for Voter {
    type DbUser = DbVoter;

    fn rights() -> Rights {
        Rights::Voter
    }
}

impl User for Admin {
    type DbUser = DbAdmin;

    fn rights() -> Rights {
        Rights::Admin
    }
}

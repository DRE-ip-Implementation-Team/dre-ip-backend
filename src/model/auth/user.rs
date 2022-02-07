use std::fmt::Display;

use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::model::{admin::Admin, mongodb::DbEntity, voter::Voter};

/// A user of our application, having defined rights.
pub trait User: DbEntity {
    /// The rights of this user type.
    const RIGHTS: Rights;
}

/// Different privilege levels.
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
    const RIGHTS: Rights = Rights::Voter;
}

impl User for Admin {
    const RIGHTS: Rights = Rights::Admin;
}

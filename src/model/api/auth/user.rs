use std::fmt::Display;

use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::model::{
    db::{admin::Admin, voter::Voter},
    mongodb::Id,
};

/// A user of our application, having defined rights.
pub trait User {
    /// The rights of this user type.
    const RIGHTS: Rights;
    /// Get the user's ID.
    fn id(&self) -> Id;
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

    fn id(&self) -> Id {
        self.id
    }
}

impl User for Admin {
    const RIGHTS: Rights = Rights::Admin;

    fn id(&self) -> Id {
        self.id
    }
}

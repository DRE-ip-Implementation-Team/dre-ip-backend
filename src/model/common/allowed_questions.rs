use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::{common::election::QuestionId, mongodb::serde_string_map};

/// The questions that a voter may answer for a particular election.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AllowedQuestions {
    /// For each allowed question, have they already confirmed a ballot?
    #[serde(with = "serde_string_map")]
    pub confirmed: HashMap<QuestionId, bool>,
}

impl Deref for AllowedQuestions {
    type Target = HashMap<QuestionId, bool>;

    fn deref(&self) -> &Self::Target {
        &self.confirmed
    }
}

impl DerefMut for AllowedQuestions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.confirmed
    }
}

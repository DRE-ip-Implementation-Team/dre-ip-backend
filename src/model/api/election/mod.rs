use crate::model::common::election::DreipGroup;

mod desc;
mod spec;

pub use desc::{ElectionCrypto, ElectionDescription, ElectionSummary};
pub use spec::{ElectionSpec, QuestionSpec};

/// Convenience wrapper for the internal election results type.
pub type ElectionResults = dre_ip::ElectionResults<String, String, DreipGroup>;

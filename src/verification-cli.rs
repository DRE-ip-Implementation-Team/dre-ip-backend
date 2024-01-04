//! A simple CLI tool for verifying DRE-ip elections.
//! This uses the internal server verification implementation, and is by definition
//! compatible with the output of our API endpoints.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;

use clap::{Arg, ArgAction, ArgMatches, Command};
use dre_ip::{DreipGroup as DreipGroupTrait, Serializable};
use rocket::serde::json::serde_json;

use dreip_backend::model::{
    api::election::{BallotError, ElectionResults, ReceiptError, VerificationError, VoteError},
    common::election::DreipGroup,
};

const PROGRAM_NAME: &str = "verify-dreip";

const ABOUT_TEXT: &str = "Verify the integrity of a DRE-ip election.

EXIT CODES:
     0: Verification succeeded.
   255: Ran successfully, but verification failed.
 Other: Error.";

const RESULTS_PATH: &str = "RESULTS_PATH";

const RESULTS_PATH_HELP: &str = "The path to a JSON dump of a specific question,\n\
as returned by `GET /elections/<election_id>/<question_id>/dump`";

/// Construct the CLI configuration.
fn cli() -> Command {
    // Make the build dirty when the toml changes.
    include_str!("../Cargo.toml");

    clap::command!(PROGRAM_NAME).about(ABOUT_TEXT).arg(
        Arg::new(RESULTS_PATH)
            .help(RESULTS_PATH_HELP)
            .action(ArgAction::Set)
            .required(true),
    )
}

/// Errors that this program may produce.
#[derive(Debug, Eq, PartialEq)]
enum Error {
    /// IO error described by the inner message.
    IO(String),
    /// Failed to decode the JSON dump.
    Format(String),
    /// Verification failed due to the contained reason.
    Verification(VerificationError),
}

/// A friendly, u64-based representation of the results for a particular candidate.
#[derive(Debug, Eq, PartialEq)]
struct FriendlyResults {
    pub candidate_name: String,
    pub tally: Option<u64>, // Might not be present if election still running.
    pub audited_votes: u64,
}

impl FriendlyResults {
    pub fn new(name: String) -> Self {
        Self {
            candidate_name: name,
            tally: None,
            audited_votes: 0,
        }
    }
}

impl Display for FriendlyResults {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(tally) = self.tally {
            write!(
                f,
                "{}: {} vote{} ({} audited ballot{})",
                self.candidate_name,
                tally,
                if tally != 1 { "s" } else { "" },
                self.audited_votes,
                if self.audited_votes != 1 { "s" } else { "" }
            )
        } else {
            write!(
                f,
                "{}: tally not available yet ({} audited ballot{} so far)",
                self.candidate_name,
                self.audited_votes,
                if self.audited_votes != 1 { "s" } else { "" }
            )
        }
    }
}

/// Convert a tally `Scalar` to a u64. We assume that it fits.
fn tally_to_u64(tally_scalar: <DreipGroup as DreipGroupTrait>::Scalar) -> u64 {
    // Convert to bytes.
    let bytes = Serializable::to_bytes(&tally_scalar);

    // Check it fits into a u64.
    const BYTES: isize = 8;
    let extra_bytes = isize::try_from(bytes.len()).expect("unreasonably big scalar") - BYTES;

    let u64_bytes: [u8; 8] = match extra_bytes.cmp(&0) {
        Ordering::Less => {
            // We can pad.
            let mut u64_bytes = [0; 8];
            let padding = (-extra_bytes) as usize;
            for (i, byte) in bytes.iter().enumerate() {
                u64_bytes[i + padding] = *byte;
            }
            u64_bytes
        }
        Ordering::Equal => {
            // We have exactly the right amount.
            bytes[..].try_into().unwrap()
        }
        Ordering::Greater => {
            for byte in &bytes[0..extra_bytes as usize] {
                if *byte != 0 {
                    // Too big for u64!
                    panic!("Tally was so large that it didn't fit into 64 bits!")
                }
            }
            // Excess bytes are all zero; we can just trim.
            let start_index = bytes.len() - 8;
            bytes[start_index..].try_into().unwrap()
        }
    };

    u64::from_be_bytes(u64_bytes)
}

/// Run verification.
fn verify(path: &str) -> Result<Vec<FriendlyResults>, Error> {
    // Load the file.
    let file = BufReader::new(File::open(path).map_err(|e| Error::IO(e.to_string()))?);
    let results: ElectionResults =
        serde_json::from_reader(file).map_err(|e| Error::Format(e.to_string()))?;

    // Run verification.
    results.verify().map_err(Error::Verification)?;

    // Assemble the friendly results.
    // First, find all the candidates.
    let candidates: Vec<String> = results
        .confirmed // We might find the list in a confirmed ballot...
        .values()
        .next()
        .map(|receipt| receipt.crypto.votes.keys().cloned().collect())
        .or_else(|| {
            results
                .audited // ...or in an audited ballot...
                .values()
                .next()
                .map(|receipt| receipt.crypto.votes.keys().cloned().collect())
        })
        .or_else(|| {
            results
                .totals // ...or in the tallies.
                .as_ref()
                .map(|totals| totals.keys().cloned().collect())
        })
        .unwrap_or_default(); // Otherwise, we have no data.

    // Create the results for each candidate.
    let mut friendly_results = HashMap::with_capacity(candidates.len());
    for candidate in candidates {
        friendly_results.insert(candidate.clone(), FriendlyResults::new(candidate));
    }
    // Count audited votes.
    for receipt in results.audited.values() {
        // Unwrap safe as we've already created all candidate entries.
        friendly_results
            .get_mut(&receipt.state_data.candidate)
            .unwrap()
            .audited_votes += 1;
    }
    // Plug in the tallies if present.
    if let Some(tallies) = &results.totals {
        for totals_desc in tallies.values() {
            // Unwrap safe as we've already created all candidate entries.
            friendly_results
                .get_mut(&totals_desc.candidate_name)
                .unwrap()
                .tally = Some(tally_to_u64(totals_desc.tally));
        }
        // If there were no confirmed votes, the tallies will be missing.
        for res in friendly_results.values_mut() {
            if res.tally.is_none() {
                res.tally = Some(0);
            }
        }
    }

    // Turn into a list ordered by tally, then name.
    let mut results_list = friendly_results.into_values().collect::<Vec<_>>();
    results_list.sort_unstable_by(|a, b| a.candidate_name.cmp(&b.candidate_name));
    if results.totals.is_some() {
        // Unwrap is safe, as if `results.totals` exists, all tallies are filled in.
        results_list.sort_by(|a, b| b.tally.unwrap().cmp(&a.tally.unwrap()));
    }

    Ok(results_list)
}

/// Run verification, report the result, and return the exit code.
fn run(args: &ArgMatches) -> u8 {
    let path: &String = args.get_one(RESULTS_PATH).unwrap(); // Required argument is guaranteed to be present.
    match verify(path) {
        Ok(friendly_results) => {
            println!("Verification succeeded.");
            for result in friendly_results {
                println!("{}", result);
            }
            0
        }
        Err(Error::IO(msg)) => {
            println!("IO error: {}", msg);
            1
        }
        Err(Error::Format(msg)) => {
            println!("Invalid JSON: {}", msg);
            1
        }
        Err(Error::Verification(err)) => {
            let msg = match err {
                VerificationError::Ballot(err) => match err {
                    BallotError::Vote(VoteError {
                        ballot_id,
                        candidate_id,
                    }) => {
                        format!(
                            "Ballot {} has an invalid vote for candidate {}.",
                            ballot_id, candidate_id
                        )
                    }
                    BallotError::BallotProof { ballot_id } => {
                        format!(
                            "Ballot {} has an invalid proof of well-formedness.",
                            ballot_id
                        )
                    }
                },
                VerificationError::Tally { candidate_id } => {
                    format!("The tally for candidate {} is incorrect.", candidate_id)
                }
                VerificationError::WrongCandidates => String::from(
                    "The candidates listed in the tallies do \
                    not match those found in the ballots.",
                ),
                VerificationError::Receipt(err) => match err {
                    ReceiptError::Signature { ballot_id } => {
                        format!(
                            "The receipt for ballot {} has an invalid signature.",
                            ballot_id
                        )
                    }
                    ReceiptError::ConfirmationCode { ballot_id } => {
                        format!(
                            "The receipt for ballot {} has an invalid confirmation code.",
                            ballot_id
                        )
                    }
                    ReceiptError::RevealedCandidate {
                        ballot_id,
                        claimed_candidate,
                        true_candidate,
                    } => {
                        format!(
                            "The receipt for ballot {} claims candidate {} but is actually for candidate {}.",
                            ballot_id,
                            claimed_candidate,
                            true_candidate
                        )
                    }
                },
            };
            println!("Verification failed: {}", msg);
            255
        }
    }
}

fn main() {
    let args = cli().get_matches();
    let exit_code = run(&args);
    std::process::exit(exit_code.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification() {
        // This test actually enters backend code, so enable logging.
        log4rs_test_utils::test_logging::init_logging_once_for(
            ["dre_ip", "dreip_backend"],
            None,
            None,
        );

        let expected_results = vec![
            FriendlyResults {
                candidate_name: "Chris Riches".to_string(),
                tally: Some(3),
                audited_votes: 1,
            },
            FriendlyResults {
                candidate_name: "Parry Hotter".to_string(),
                tally: Some(2),
                audited_votes: 1,
            },
        ];
        assert_eq!(verify("example_dumps/election.json"), Ok(expected_results));

        let expected_results = vec![
            FriendlyResults {
                candidate_name: "Chris Riches".to_string(),
                tally: None,
                audited_votes: 0,
            },
            FriendlyResults {
                candidate_name: "Parry Hotter".to_string(),
                tally: None,
                audited_votes: 1,
            },
        ];
        assert_eq!(
            verify("example_dumps/election_inprogress.json"),
            Ok(expected_results)
        );

        assert_eq!(
            verify("example_dumps/election_invalid_candidate.json"),
            Err(Error::Verification(VerificationError::Receipt(
                ReceiptError::RevealedCandidate {
                    ballot_id: 11,
                    claimed_candidate: "Chris Riches".to_string(),
                    true_candidate: "Parry Hotter".to_string(),
                }
            )))
        );
        assert_eq!(
            verify("example_dumps/election_invalid_conf_code.json"),
            Err(Error::Verification(VerificationError::Receipt(
                ReceiptError::ConfirmationCode { ballot_id: 11 }
            )))
        );
        assert_eq!(
            verify("example_dumps/election_invalid_signature.json"),
            Err(Error::Verification(VerificationError::Receipt(
                ReceiptError::Signature { ballot_id: 5 }
            )))
        );
        assert_eq!(
            verify("example_dumps/election_invalid_totals.json"),
            Err(Error::Verification(VerificationError::Tally {
                candidate_id: "Parry Hotter".into()
            }))
        );
    }

    #[test]
    fn correct_cli_usage() {
        let command_line = [PROGRAM_NAME, "example_dumps/election.json"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 0);

        let command_line = [PROGRAM_NAME, "example_dumps/election_invalid_totals.json"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 255);

        let command_line = [PROGRAM_NAME, "example_dumps/election_malformed.json"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);

        let command_line = [PROGRAM_NAME, "not a real file"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);
    }

    #[test]
    fn bad_cli_usage() {
        // Something very wrong.
        let command_line = [PROGRAM_NAME, "this", "invocation", "is", "incorrect"];
        cli().try_get_matches_from(command_line).unwrap_err();

        // No options at all.
        let command_line = [PROGRAM_NAME];
        cli().try_get_matches_from(command_line).unwrap_err();
    }
}

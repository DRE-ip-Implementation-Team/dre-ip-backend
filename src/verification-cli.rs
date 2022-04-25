//! A simple CLI tool for verifying DRE-ip elections.
//! This uses the internal server verification implementation, and is by definition
//! compatible with the output of our API endpoints.

use std::fs::File;
use std::io::BufReader;

use clap::{Arg, ArgMatches, Command};
use rocket::serde::json::serde_json;

use dreip_backend::model::api::election::{
    BallotError, ElectionResults, ReceiptError, VerificationError, VoteError,
};

const PROGRAM_NAME: &str = "verify-dreip";

const ABOUT_TEXT: &str = "Verify the integrity of a DRE-ip election.

EXIT CODES:
     0: Verification succeeded.
   255: Ran successfully, but verification failed.
 Other: Error.";

const RESULTS_PATH: &str = "RESULTS_PATH";

const RESULTS_PATH_HELP: &str = "The path to a JSON dump of a specific question, \
as returned by `GET /elections/<election_id>/<question_id>/dump`";

/// Construct the CLI configuration.
fn cli() -> Command<'static> {
    // Make the build dirty when the toml changes.
    include_str!("../Cargo.toml");

    clap::command!(PROGRAM_NAME).about(ABOUT_TEXT).arg(
        Arg::new(RESULTS_PATH)
            .help(RESULTS_PATH_HELP)
            .takes_value(true)
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
    /// Verification failed for the described reason.
    Verification(VerificationError),
}

/// Run verification.
fn verify(path: &str) -> Result<(), Error> {
    let file = BufReader::new(File::open(path).map_err(|e| Error::IO(e.to_string()))?);
    let results: ElectionResults =
        serde_json::from_reader(file).map_err(|e| Error::Format(e.to_string()))?;
    results.verify().map_err(Error::Verification)
}

/// Run verification, report the result, and return the exit code.
fn run(args: &ArgMatches) -> u8 {
    let path = args.value_of(RESULTS_PATH).unwrap(); // Required argument is guaranteed to be present.
    match verify(path) {
        Ok(()) => {
            println!("Verification succeeded.");
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
        assert!(verify("example_dumps/election.json").is_ok());
        assert!(verify("example_dumps/election_inprogress.json").is_ok());

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

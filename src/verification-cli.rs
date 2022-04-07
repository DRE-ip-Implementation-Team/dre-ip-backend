//! A simple CLI tool for verifying an election dump.
//! This uses the internal server verification implementation, and is by definition
//! compatible with the dumps produced by the question dump endpoint.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use clap::Parser;
use rocket::serde::json::serde_json;

use dreip_backend::model::api::election::{
    BallotError, ElectionResults, VerificationError, VoteError,
};

/// Shown in the help message.
const ABOUT_TEXT: &str = "Verify the integrity of a DRE-ip election using the P256 elliptic curve.

Exit codes:
     0: Success
   255: Ran successfully, but election failed to verify.
 Other: Error";

/// The CLI arguments to parse.
#[derive(Debug, Parser)]
#[clap(name = "verify-election", author, version,
about = ABOUT_TEXT, long_about = None)]
struct Args {
    /// The JSON election dump to verify.
    file: String,
}

/// Errors that this program may produce.
#[derive(Debug, Eq, PartialEq)]
enum Error {
    /// IO error described by the inner message.
    IO(String),
    /// Failed to decode the election dump.
    Format(String),
    /// Verification failed for the described reason.
    Verification(VerificationError),
}

fn verify(path: impl AsRef<Path>) -> Result<(), Error> {
    // Try to load the file.
    let file = File::open(path).map_err(|e| Error::IO(e.to_string()))?;
    // Try to read the election dump.
    let results: ElectionResults =
        serde_json::from_reader(BufReader::new(file)).map_err(|e| Error::Format(e.to_string()))?;
    // Verify the election.
    results.verify().map_err(|e| Error::Verification(e))
}

fn run(args: &Args) -> u8 {
    match verify(&args.file) {
        Ok(()) => {
            println!("Election successfully verified.");
            0
        }
        Err(Error::IO(msg)) => {
            println!("IO error: {}", msg);
            1
        }
        Err(Error::Format(msg)) => {
            println!("Invalid election dump: {}", msg);
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
                VerificationError::Receipt { ballot_id } => {
                    format!(
                        "The receipt for ballot {} has an invalid signature.",
                        ballot_id
                    )
                }
            };
            println!("Election failed to verify: {}", msg);
            255
        }
    }
}

fn main() {
    let args: Args = Args::parse();
    let exit_code: u8 = run(&args);
    std::process::exit(exit_code.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification() {
        assert!(verify("example_election_dumps/election.json").is_ok());
        assert_eq!(
            verify("example_election_dumps/election_invalid.json"),
            Err(Error::Verification(VerificationError::Tally {
                candidate_id: "Parry Hotter".into()
            }))
        );
    }

    #[test]
    fn test_cli() {
        let cli = ["verify-election", "example_election_dumps/election.json"];
        let args: Args = Args::try_parse_from(cli).unwrap();
        assert_eq!(run(&args), 0);

        let cli = [
            "verify-election",
            "example_election_dumps/election_invalid.json",
        ];
        let args: Args = Args::try_parse_from(cli).unwrap();
        assert_eq!(run(&args), 255);

        let cli = [
            "verify-election",
            "example_election_dumps/election_malformed.json",
        ];
        let args: Args = Args::try_parse_from(cli).unwrap();
        assert_eq!(run(&args), 1);

        let cli = ["verify-election", "not a real file"];
        let args: Args = Args::try_parse_from(cli).unwrap();
        assert_eq!(run(&args), 1);

        let cli = ["verify-election", "this", "invocation", "is", "incorrect"];
        Args::try_parse_from(cli).unwrap_err();
    }
}

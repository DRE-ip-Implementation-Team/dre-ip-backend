//! A simple CLI tool for verifying DRE-ip elections.
//! This uses the internal server verification implementation, and is by definition
//! compatible with the output of our API endpoints.

use std::fs::File;
use std::io::BufReader;

use clap::{Arg, ArgMatches, Command};
use rocket::serde::json::serde_json;
use serde::Deserialize;

use dreip_backend::model::{
    api::{
        election::{
            verify_receipt, BallotError, ElectionCrypto, ElectionResults, VerificationError,
            VoteError,
        },
        receipt::Receipt,
    },
    db::ballot::{Audited, Confirmed, Unconfirmed},
};

const PROGRAM_NAME: &str = "verify-dreip";

const ABOUT_TEXT: &str = "Verify the integrity of a DRE-ip election or ballot (using the P256 elliptic curve).

Use either the -f option to verify an entire question's results, or the -r and -c options to verify an individual ballot's receipt.

EXIT CODES:
     0: Verification succeeded.
   255: Ran successfully, but verification failed.
 Other: Error.";

const FULL_RESULTS: &str = "FULL_RESULTS_PATH";
const RECEIPT: &str = "RECEIPT_PATH";
const CRYPTO: &str = "CRYPTO_PATH";

const FULL_RESULTS_HELP: &str = "The path to a JSON dump of the full election \
results for a specific question, as returned by `GET /elections/<election_id>/\
<question_id>/dump`. Specify this to verify an entire question's ballots and results.";

const RECEIPT_HELP: &str = "The path to a JSON dump of a single receipt, as returned \
when casting, auditing, or confirming a ballot. Specify this to verify a single ballot \
independent of its question. Requires CRYPTO_PATH to be specified as well.";

const CRYPTO_HELP: &str = "The path to a JSON dump of the election cryptographic \
configuration, i.e. an object containing `g1`, `g2`, and `public_key`. For \
compatibility with the values returned by `GET /elections/<election_id>`, these three \
fields may be nested inside an object called `crypto`. \
This file is only needed when using the -r option to verify an individual receipt.";

/// Construct the CLI configuration.
fn cli() -> Command<'static> {
    // Make the build dirty when the toml changes.
    include_str!("../Cargo.toml");

    clap::command!(PROGRAM_NAME)
        .about(ABOUT_TEXT)
        .arg(
            Arg::new(FULL_RESULTS)
                .short('f')
                .long("full-results")
                .help(FULL_RESULTS_HELP)
                .takes_value(true)
                .required_unless_present(RECEIPT)
                .conflicts_with_all(&[RECEIPT, CRYPTO]),
        )
        .arg(
            Arg::new(RECEIPT)
                .short('r')
                .long("receipt")
                .help(RECEIPT_HELP)
                .takes_value(true)
                .required_unless_present(FULL_RESULTS)
                .requires(CRYPTO),
        )
        .arg(
            Arg::new(CRYPTO)
                .short('c')
                .long("crypto")
                .help(CRYPTO_HELP)
                .takes_value(true),
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

/// A verification task for us to complete.
#[derive(Debug)]
enum VerificationTask<'a> {
    FullResults {
        path: &'a str,
    },
    Receipt {
        receipt_path: &'a str,
        crypto_path: &'a str,
    },
}

/// A receipt that is either Confirmed or Audited.
/// With the untagged representation, we can deserialize any `Receipt<S: BallotState>`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AnyReceipt {
    Audited(Receipt<Audited>),
    Confirmed(Receipt<Confirmed>),
    Unconfirmed(Receipt<Unconfirmed>),
}

impl AnyReceipt {
    /// Verify this receipt.
    pub fn verify(&self, crypto: &ElectionCrypto) -> Result<(), VerificationError> {
        match self {
            AnyReceipt::Audited(receipt) => verify_receipt(receipt, crypto),
            AnyReceipt::Confirmed(receipt) => verify_receipt(receipt, crypto),
            AnyReceipt::Unconfirmed(receipt) => verify_receipt(receipt, crypto),
        }
    }
}

/// Election crypto, optionally nested.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Crypto {
    TopLevel(ElectionCrypto),
    Nested { crypto: ElectionCrypto },
}

impl From<Crypto> for ElectionCrypto {
    /// Unwrap the possibly-nested `ElectionCrypto`.
    fn from(crypto: Crypto) -> Self {
        match crypto {
            Crypto::TopLevel(c) => c,
            Crypto::Nested { crypto } => crypto,
        }
    }
}

/// Read the given file path, and read its results in as JSON of the specified type.
fn load_path<T>(path: &str) -> Result<T, Error>
where
    T: for<'de> Deserialize<'de>,
{
    let file = BufReader::new(File::open(path).map_err(|e| Error::IO(e.to_string()))?);
    serde_json::from_reader(file).map_err(|e| Error::Format(e.to_string()))
}

/// Run verification.
fn verify(task: &VerificationTask) -> Result<(), Error> {
    let verification_result = match task {
        VerificationTask::FullResults { path } => {
            let results: ElectionResults = load_path(path)?;
            results.verify()
        }
        VerificationTask::Receipt {
            receipt_path,
            crypto_path,
        } => {
            let receipt: AnyReceipt = load_path(receipt_path)?;
            let crypto: Crypto = load_path(crypto_path)?;
            receipt.verify(&crypto.into())
        }
    };
    verification_result.map_err(Error::Verification)
}

/// Run verification, report the result, and return the exit code.
fn run(args: &ArgMatches) -> u8 {
    let task = if args.is_present(FULL_RESULTS) {
        VerificationTask::FullResults {
            path: args.value_of(FULL_RESULTS).unwrap(),
        }
    } else {
        VerificationTask::Receipt {
            receipt_path: args.value_of(RECEIPT).unwrap(),
            crypto_path: args.value_of(CRYPTO).unwrap(),
        }
    };

    match verify(&task) {
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
                VerificationError::Receipt { ballot_id } => {
                    format!(
                        "The receipt for ballot {} has an invalid signature.",
                        ballot_id
                    )
                }
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
    fn full_verification() {
        assert!(verify(&VerificationTask::FullResults {
            path: "example_dumps/election.json"
        })
        .is_ok());
        assert_eq!(
            verify(&VerificationTask::FullResults {
                path: "example_dumps/election_invalid.json"
            }),
            Err(Error::Verification(VerificationError::Tally {
                candidate_id: "Parry Hotter".into()
            }))
        );
    }

    #[test]
    fn receipt_verification() {
        assert!(verify(&VerificationTask::Receipt {
            receipt_path: "example_dumps/audited_receipt.json",
            crypto_path: "example_dumps/election_dump.json"
        })
        .is_ok());
        assert!(verify(&VerificationTask::Receipt {
            receipt_path: "example_dumps/confirmed_receipt.json",
            crypto_path: "example_dumps/crypto_dump.json"
        })
        .is_ok());
        assert_eq!(
            verify(&VerificationTask::Receipt {
                receipt_path: "example_dumps/audited_receipt_invalid.json",
                crypto_path: "example_dumps/crypto_dump.json"
            }),
            Err(Error::Verification(VerificationError::Receipt {
                ballot_id: "622650f453036aff34eb72b1".parse().unwrap(),
            }))
        );
        assert_eq!(
            verify(&VerificationTask::Receipt {
                receipt_path: "example_dumps/confirmed_receipt_invalid.json",
                crypto_path: "example_dumps/crypto_dump.json"
            }),
            Err(Error::Verification(VerificationError::Receipt {
                ballot_id: "622650f453036aff34eb72a9".parse().unwrap(),
            }))
        );
    }

    #[test]
    fn correct_cli_usage() {
        // Full question usage.
        let command_line = [PROGRAM_NAME, "-f", "example_dumps/election.json"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 0);

        let command_line = [PROGRAM_NAME, "-f", "example_dumps/election_invalid.json"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 255);

        let command_line = [PROGRAM_NAME, "-f", "example_dumps/election_malformed.json"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);

        let command_line = [PROGRAM_NAME, "-f", "not a real file"];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);

        // Individual receipt usage.
        let command_line = [
            PROGRAM_NAME,
            "-r",
            "example_dumps/audited_receipt.json",
            "-c",
            "example_dumps/crypto_dump.json",
        ];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 0);

        let command_line = [
            PROGRAM_NAME,
            "-r",
            "example_dumps/audited_receipt_invalid.json",
            "-c",
            "example_dumps/crypto_dump.json",
        ];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 255);

        let command_line = [
            PROGRAM_NAME,
            "-r",
            "example_dumps/audited_receipt.json",
            "-c",
            "example_dumps/crypto_dump_malformed.json",
        ];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);

        let command_line = [
            PROGRAM_NAME,
            "-r",
            "not a real file",
            "-c",
            "example_dumps/crypto_dump.json",
        ];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);

        // Mixing up the arguments.
        let command_line = [
            PROGRAM_NAME,
            "-c",
            "example_dumps/audited_receipt.json",
            "-r",
            "example_dumps/crypto_dump_malformed.json",
        ];
        let args = cli().try_get_matches_from(command_line).unwrap();
        assert_eq!(run(&args), 1);
    }

    #[test]
    fn bad_cli_usage() {
        // Something very wrong.
        let command_line = [PROGRAM_NAME, "this", "invocation", "is", "incorrect"];
        cli().try_get_matches_from(command_line).unwrap_err();

        // Mixing modes of operation.
        let command_line = [
            PROGRAM_NAME,
            "-f",
            "example_dumps/election.json",
            "-r",
            "example_dumps/audited_receipt.json",
            "-c",
            "example_dumps/crypto_dump.json",
        ];
        cli().try_get_matches_from(command_line).unwrap_err();

        // Receipt without crypto.
        let command_line = [PROGRAM_NAME, "-r", "example_dumps/audited_receipt.json"];
        cli().try_get_matches_from(command_line).unwrap_err();

        // Crypto without receipt.
        let command_line = [PROGRAM_NAME, "-c", "example_dumps/crypto_dump.json"];
        cli().try_get_matches_from(command_line).unwrap_err();

        // No options at all.
        let command_line = [PROGRAM_NAME];
        cli().try_get_matches_from(command_line).unwrap_err();
    }
}

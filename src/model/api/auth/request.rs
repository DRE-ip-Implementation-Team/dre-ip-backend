#[cfg_attr(test, allow(unused_imports))]
use chrono::{DateTime, Duration, Utc};
use reqwest;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::api::sms::Sms;

#[cfg(test)]
const TEST_RECAPTCHA_RESPONSE: &str = "this response will succeed in test mode";

/// reCAPTCHA tokens older than this many minutes are not accepted.
#[cfg_attr(test, allow(dead_code))]
const MAX_TOKEN_LIFE_MINUTES: i64 = 3;

/// An authentication request for a specific SMS number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    sms: Sms, // Deliberately not public, so it can only be extracted via `verify()`
    g_recaptcha_response: String,
}

impl AuthRequest {
    /// Verify the reCAPTCHA, revealing the SMS if successful.
    /// This can only be attempted once, due to the reCAPTCHA API.
    #[cfg_attr(test, allow(unused_variables))]
    pub async fn verify(self, secret: &str, hostname: &str) -> Result<Sms, RecaptchaError> {
        // In test mode, just check the dummy value is equal to some string.
        #[cfg(test)]
        if self.g_recaptcha_response == TEST_RECAPTCHA_RESPONSE {
            Ok(self.sms)
        } else {
            Err(RecaptchaError::InvalidToken)
        }
        // When doing it for real, contact the google API.
        #[cfg(not(test))]
        {
            let client = reqwest::Client::new();
            let parameters = RecaptchaVerifyRequest {
                secret: secret.to_string(),
                response: self.g_recaptcha_response,
            };
            let response: RecaptchaVerifyResponse = client
                .post("https://www.google.com/recaptcha/api/siteverify")
                .form(&parameters)
                .send()
                .await
                .map_err(RecaptchaError::ConnectionError)?
                .json()
                .await
                .map_err(RecaptchaError::ConnectionError)?;

            if !response.success || !response.error_codes.is_empty() {
                return Err(RecaptchaError::InvalidToken);
            }
            // Otherwise, we expect the other fields to be present.
            let timestamp = response
                .challenge_ts
                .expect("challenge_ts was not present when success was true");
            if timestamp + Duration::minutes(MAX_TOKEN_LIFE_MINUTES) < Utc::now() {
                return Err(RecaptchaError::OldToken);
            }
            let actual_hostname = response
                .hostname
                .expect("hostname was not present when success was true");
            if actual_hostname != hostname {
                Err(RecaptchaError::WrongHostname(actual_hostname))
            } else {
                Ok(self.sms)
            }
        }
    }
}

/// Possible errors resulting from verifying a reCAPTCHA token.
#[derive(Debug, Error)]
pub enum RecaptchaError {
    /// Failed to contact the google verification API.
    #[error("Failed to contact reCAPTCHA verification. Details: {0}")]
    ConnectionError(#[from] reqwest::Error),
    /// The token came back with errors.
    #[error("Invalid reCAPTCHA")]
    InvalidToken,
    /// The token was more than `MAX_TOKEN_LIFE` minutes old.
    #[error("Invalid reCAPTCHA (too old)")]
    OldToken,
    #[error("Invalid reCAPTCHA (bad hostname '{0}')")]
    /// The token came from the wrong site.
    WrongHostname(String),
}

/// A reCAPTCHA verification request to send to the google API.
#[derive(Serialize, Deserialize)]
struct RecaptchaVerifyRequest {
    /// API connection key.
    pub secret: String,
    /// The reCAPTCHA token from the client.
    pub response: String,
}

/// A reCAPTCHA verification response from the google API.
#[derive(Serialize, Deserialize)]
struct RecaptchaVerifyResponse {
    /// Did the reCAPTCHA successfully verify?
    pub success: bool,
    /// When was the challenge loaded?
    pub challenge_ts: Option<DateTime<Utc>>,
    /// What was the hostname of the site where the reCAPTCHA was solved?
    pub hostname: Option<String>,
    /// Any error codes.
    #[serde(default)]
    pub error_codes: Vec<String>,
}

#[cfg(test)]
mod examples {
    use super::*;

    impl AuthRequest {
        pub fn example() -> Self {
            Self {
                sms: Sms::example(),
                g_recaptcha_response: TEST_RECAPTCHA_RESPONSE.to_string(),
            }
        }

        pub fn example_invalid() -> Self {
            Self {
                sms: Sms::example(),
                g_recaptcha_response: "not valid".to_string(),
            }
        }
    }
}

#[cfg_attr(test, allow(unused_imports))]
use chrono::{DateTime, Duration, Utc};
use rocket::http::Status;
use serde::{Deserialize, Serialize};

use crate::{error::Error, model::api::sms::Sms};

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
                .map_err(|_| RecaptchaError::ConnectionError)?
                .json()
                .await
                .map_err(|_| RecaptchaError::ConnectionError)?;

            if !response.success || !response.error_codes.is_empty() {
                Err(RecaptchaError::InvalidToken)
            } else if response.challenge_ts + Duration::minutes(MAX_TOKEN_LIFE_MINUTES) > Utc::now()
            {
                Err(RecaptchaError::OldToken)
            } else if response.hostname != hostname {
                Err(RecaptchaError::WrongHostname(response.hostname))
            } else {
                Ok(self.sms)
            }
        }
    }
}

/// Possible errors resulting from verifying a reCAPTCHA token.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RecaptchaError {
    /// Failed to contact the google verification API.
    ConnectionError,
    /// The token came back with errors.
    InvalidToken,
    /// The token was more than MAX_TOKEN_LIFE minutes old.
    OldToken,
    /// The token came from the wrong site.
    WrongHostname(String),
}

impl From<RecaptchaError> for Error {
    fn from(err: RecaptchaError) -> Self {
        match err {
            RecaptchaError::ConnectionError => Error::Status(
                Status::InternalServerError,
                "Failed to contact reCAPTCHA verification".to_string(),
            ),
            RecaptchaError::InvalidToken => {
                Error::Status(Status::Unauthorized, "Invalid reCAPTCHA".to_string())
            }
            RecaptchaError::OldToken => Error::Status(
                Status::Unauthorized,
                "Invalid reCAPTCHA (too old)".to_string(),
            ),
            RecaptchaError::WrongHostname(hostname) => Error::Status(
                Status::Unauthorized,
                format!("Invalid reCAPTCHA (bad hostname '{}')", hostname),
            ),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RecaptchaVerifyRequest {
    pub secret: String,
    pub response: String,
}

#[derive(Serialize, Deserialize)]
struct RecaptchaVerifyResponse {
    pub success: bool,
    pub challenge_ts: DateTime<Utc>,
    pub hostname: String,
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

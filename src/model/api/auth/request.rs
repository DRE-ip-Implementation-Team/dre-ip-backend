use serde::{Deserialize, Serialize};

use crate::model::api::sms::Sms;

#[cfg(test)]
const TEST_CAPTCHA_RESPONSE: &str = "this response will succeed in test mode";

/// An authentication request for a specific SMS number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    sms: Sms, // Deliberately not public, so it can only be extracted via `verify()`
    g_captcha_response: String,
}

impl AuthRequest {
    /// Verify the CAPTCHA, revealing the SMS if successful.
    /// This can only be attempted once, due to the reCAPTCHA API.
    pub async fn verify(self) -> Option<Sms> {
        // In test mode, just check the dummy value is equal to some string.
        #[cfg(test)]
        if self.g_captcha_response == TEST_CAPTCHA_RESPONSE {
            Some(self.sms)
        } else {
            None
        }
        // When doing it for real, contact the google API.
        #[cfg(not(test))]
        {
            todo!()
        }
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    impl AuthRequest {
        pub fn example() -> Self {
            Self {
                sms: Sms::example(),
                g_captcha_response: TEST_CAPTCHA_RESPONSE.to_string(),
            }
        }

        pub fn example_invalid() -> Self {
            Self {
                sms: Sms::example(),
                g_captcha_response: "not valid".to_string(),
            }
        }
    }
}

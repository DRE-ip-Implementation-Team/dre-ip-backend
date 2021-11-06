use rocket::{http::{CookieJar, Status}, Route, serde::json::Json};
use serde::Deserialize;

pub fn routes() -> Vec<Route> {
    routes![
        login_admin,
        login_voter_request_otp,
        login_voter_submit_otp,
    ]
}

#[post("/login/admin", format="json", data="<login>")]
fn login_admin(cookies: &CookieJar, login: Json<AdminLogin>) -> Status {
    Status::ImATeapot  // TODO
}

#[post("/login/voter/request-otp", format="json", data="<sms>")]
fn login_voter_request_otp(cookies: &CookieJar, sms: Json<VoterSMSNumber>) -> Status {
    Status::ImATeapot  // TODO
}

#[post("/login/voter/submit-otp", format="json", data="<otp>")]
fn login_voter_submit_otp(cookies: &CookieJar, otp: Json<VoterOTP>) -> Status {
    Status::ImATeapot  // TODO
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdminLogin {
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoterSMSNumber {
    sms: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VoterOTP {
    sms: String,
    otp: String,
}

use rocket::{
    fairing::{Fairing, Info, Kind},
    http::StatusClass,
    request::{FromRequest, Outcome},
    Data, Orbit, Request, Response, Rocket,
};
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A unique identifier for a particular request.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct RequestId(pub usize);

impl Display for RequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl RequestId {
    /// Atomically get the next ID. This wraps around back to zero if you somehow exceed a usize.
    pub fn next() -> RequestId {
        static REQUEST_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
        RequestId(REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Allow the ID to be accessed via request guard.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r RequestId {
    type Error = (); // No errors possible, use the `!` type once stabilised.

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(req.local_cache(RequestId::next))
    }
}

/// A rocket fairing that does global logging, e.g. logging every request and response.
#[derive(Debug, Copy, Clone)]
pub struct LoggerFairing;

#[rocket::async_trait]
impl Fairing for LoggerFairing {
    fn info(&self) -> Info {
        Info {
            name: "Logger",
            kind: Kind::Liftoff | Kind::Request | Kind::Response | Kind::Shutdown,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let protocol = if rocket.config().tls_enabled() {
            "https"
        } else {
            "http"
        };
        let ip = &rocket.config().address;
        let port = &rocket.config().port;
        info!("Server launched on {protocol}://{ip}:{port}");
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        // Assign an ID.
        let id = req.local_cache(RequestId::next);
        // Get the HTTP method.
        let method = req.method();
        // Get the request URI.
        let uri = req.uri();
        // Log the incoming request.
        info!("->req{id} {method} {uri}");
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        // Get the ID.
        let id = req.local_cache(RequestId::next);
        // Get the response code.
        let code = res.status();
        // Get the matched route.
        let route = match req.route() {
            Some(r) => {
                let mut str = r.uri.to_string();
                if let Some(ref name) = r.name {
                    str = format!("{name} ({str})");
                }
                str
            }
            None => "UNKNOWN ROUTE".to_string(),
        };
        // Log the outgoing response.
        let log_msg = format!("<-rsp{id} {code} {route}");
        match code.class() {
            StatusClass::ServerError => error!("{log_msg}"),
            StatusClass::ClientError => warn!("{log_msg}"),
            _ => info!("{log_msg}"),
        }
    }

    async fn on_shutdown(&self, _rocket: &Rocket<Orbit>) {
        warn!("Shutdown requested, stopping gracefully...");
    }
}

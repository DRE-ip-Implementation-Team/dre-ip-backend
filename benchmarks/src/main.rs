use anyhow::anyhow;
use chrono::{Duration, Utc};
use clap::{Parser, ValueEnum};
use const_format::concatcp;
use rand::seq::SliceRandom;
use regex::bytes::Regex;
use reqwest::blocking::{Client, Response};
use serde::{de::IgnoredAny, Deserialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, ErrorKind, Read, Write};
use std::ops::{AddAssign, Div};
use std::process::{self, Child, Command, Stdio};
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use tempfile::NamedTempFile;

const LOCAL_PORT: u32 = 8374;
const LOCAL_URL: &str = concatcp!("http://127.0.0.1:", LOCAL_PORT);

#[rustfmt::skip]
const ROCKET_ENV: &[(&str, &str)] = &[
    ("ROCKET_PORT", concatcp!(LOCAL_PORT)),
    ("ROCKET_SECRET_KEY", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
    ("ROCKET_JWT_SECRET", "dummy"),
    ("ROCKET_RECAPTCHA_SECRET", "dummy"),
    ("ROCKET_HMAC_SECRET", "dummy"),
    ("ROCKET_AWS_REGION", "dummy"),
    ("ROCKET_AWS_ACCESS_KEY_ID", "dummy"),
    ("ROCKET_AWS_SECRET_ACCESS_KEY", "dummy"),
];

const CAPTCHA_RESPONSE: &str = "this response will succeed in test mode";

#[rustfmt::skip]
const CANDIDATES: &[&str] = &[
    "Alice",
    "Bob",
    "Carol",
    "Dave",
    "Eve",
    "Fred",
    "Grace",
    "Henry",
    "Irene",
    "Joe",
];

#[derive(Parser)]
struct Args {
    /// Silence local server logging.
    #[arg(short, long)]
    quiet: bool,

    /// Send local server logging to this file; takes precedence over --quiet.
    #[arg(long)]
    logfile: Option<String>,

    /// Suppress test-setup.sh output and always re-use any existing DB.
    #[arg(long)]
    reuse_db: bool,

    /// Connect to a remote server at this URL instead of running a local one.
    #[arg(long)]
    remote: Option<String>,

    /// How many threads to use. Defaults to the number of logical CPUs.
    #[arg(long, default_value_t = num_cpus::get())]
    threads: usize,

    /// The behaviour around confirmation / cancellation of votes.
    #[arg(long, value_enum, default_value_t)]
    confirm_mode: ConfirmMode,

    /// Verify election integrity after completion.
    #[arg(long)]
    verify: bool,
}

/// The behaviour around confirmation / cancellation of votes.
#[derive(Debug, Copy, Clone, ValueEnum)]
enum ConfirmMode {
    /// Confirm all votes.
    Confirm,
    /// Audit all votes.
    Audit,
    /// Randomly confirm or audit with equal probability.
    Random,
    /// Attempt to model real behaviour: confirm with 98% probability.
    Realistic,
}

impl ConfirmMode {
    /// Return `true` if we should confirm; `false` if we should audit.
    fn should_confirm(&self) -> bool {
        match self {
            Self::Confirm => true,
            Self::Audit => false,
            Self::Random => rand::random(),
            Self::Realistic => rand::random::<f32>() >= 0.02,
        }
    }
}

impl Default for ConfirmMode {
    fn default() -> Self {
        Self::Realistic
    }
}

/// Construct a URL from segments.
macro_rules! url {
    ($($segment:expr),+) => {{
        std::path::PathBuf::from_iter([$($segment),+]).to_str().unwrap()
    }}
}

/// Set up everything we need before starting the server.
fn setup_deps(always_reuse: bool) -> anyhow::Result<()> {
    // Ensure the optimised build is up-to-date. Turn off default features to
    // bypass OTP/CAPTCHA authentication.
    Command::new("cargo")
        .args(["build", "--release", "--no-default-features"])
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("server build exited nonzero"))?;

    // Ensure the DB is running and set its URI in the environment.
    let stdin = if always_reuse {
        Stdio::piped()
    } else {
        Stdio::inherit()
    };
    let mut test_setup = Command::new("./test-setup.sh")
        .stdin(stdin)
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(mut proc_stdin) = test_setup.stdin.take() {
        // Send an 'r' to trigger DB reuse.
        proc_stdin.write_all(b"r\n")?;
    }

    // Bounce to our stdout while scanning for the DB URI.
    let mut proc_stdout = test_setup.stdout.take().unwrap();
    let mut buf = [0u8; 128];

    let mut stdout = io::stdout();

    let mut found = false;
    let mut line = Vec::<u8>::with_capacity(256);
    let pattern = Regex::new(r"'export ROCKET_DB_URI=(.+)'").unwrap();

    loop {
        // Read a chunk of output.
        let n = match proc_stdout.read(&mut buf) {
            Ok(n) => n,
            Err(e) => {
                if e.kind() == ErrorKind::Interrupted {
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        };
        if n == 0 {
            // Pipe closed.
            break;
        }

        if !always_reuse {
            // Bounce it to stdout.
            stdout.write_all(&buf[..n])?;
            stdout.flush()?;
        }

        // Look for the DB URI. We use a separate line buffer to avoid missing the match in the
        // case where it lies across a read() boundary.
        if !found {
            // Clear previous lines out of the line buffer.
            if let Some(i) = line.iter().rposition(|b| *b == b'\n') {
                line.drain(..=i);
            }

            // Add this blob to the line buffer.
            line.extend_from_slice(&buf[..n]);

            // Look in the line buffer for a match.
            if let Some(caps) = pattern.captures(&line) {
                let val = std::str::from_utf8(&caps[1])?;
                env::set_var("ROCKET_DB_URI", val);
                found = true;
            }
        }
    }

    test_setup
        .wait()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("test-setup returned nonzero"))?;

    if !found {
        return Err(anyhow!("didn't see ROCKET_DB_URI in test-setup output"));
    }

    // Set other environment variables.
    for (var, val) in ROCKET_ENV {
        env::set_var(var, val);
    }

    Ok(())
}

/// Terminate the given child process. This is a SIGTERM on unix and a hard-kill on other
/// platforms.
fn terminate_child(child: &mut Child) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        let pid = nix::unistd::Pid::from_raw(child.id() as i32);
        nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGTERM)?;
    }
    #[cfg(not(unix))]
    {
        child.kill()?;
    }
    Ok(())
}

/// Set up everything we need before starting the server.
fn launch_server(logfile: Stdio) -> anyhow::Result<Child> {
    let mut proc = Command::new("./target/release/dreip-backend")
        .stdout(logfile)
        .spawn()?;

    // Wait for the server to be reachable.
    let client = Client::new();
    loop {
        let resp = client
            .get(url!(LOCAL_URL, "auth/check"))
            .send()
            .and_then(Response::error_for_status);

        if let Ok(resp) = resp {
            let text = resp.text();
            if let Ok("Unauthenticated") = text.as_deref() {
                break;
            } else {
                terminate_child(&mut proc)?;
                proc.wait()?;
                return Err(anyhow!("Bad response: {:?}", text));
            }
        }

        // Check the server didn't exit.
        if let Some(retcode) = proc.try_wait()? {
            return Err(anyhow!("Server exited prematurely with code {}", retcode));
        }
    }

    Ok(proc)
}

/// Create an election to benchmark against and return its ID.
fn setup_election(url: &str) -> anyhow::Result<String> {
    let client = Client::builder().cookie_store(true).build()?;

    // Authenticate as admin.
    let creds = json!({
        "username": "replace-this-admin-asap",
        "password": "insecure",
    });
    client
        .post(url!(url, "auth/admin"))
        .json(&creds)
        .send()
        .and_then(Response::error_for_status)?;

    // Create election.
    let start = Utc::now() - Duration::hours(1);
    let end = start + Duration::hours(2);
    let election = json!({
        "name": "Benchmark Election",
        "start_time": start,
        "end_time": end,
        "electorates": [],
        "questions": [{
            "description": "Benchmark Question",
            "constraints": {},
            "candidates": CANDIDATES,
        }],
    });
    let resp = client
        .post(url!(url, "elections"))
        .json(&election)
        .send()
        .and_then(Response::error_for_status)?;

    #[derive(Deserialize)]
    struct Eid {
        id: u32,
    }
    let Eid { id } = resp.json()?;
    let id_str = id.to_string();

    // Publish the election.
    client
        .post(url!(url, "elections", &id_str, "publish"))
        .send()
        .and_then(Response::error_for_status)?;

    Ok(id_str)
}

/// Authenticate as a voter and return the client with embedded auth cookies.
fn voter_auth(url: &str, voter_id: u32) -> anyhow::Result<(Client, StdDuration)> {
    let client = Client::builder().cookie_store(true).build()?;
    let start = Instant::now();

    // Challenge phase.
    let data = json!({
        "sms": format!("+1{:010}", voter_id),
        "g_recaptcha_response": CAPTCHA_RESPONSE,
    });
    client
        .post(url!(url, "auth/voter/challenge"))
        .json(&data)
        .send()
        .and_then(Response::error_for_status)?;

    // Verification phase.
    let data = json!({
        "code": "123456",
        "g_recaptcha_response": CAPTCHA_RESPONSE,
    });
    client
        .post(url!(url, "auth/voter/verify"))
        .json(&data)
        .send()
        .and_then(Response::error_for_status)?;

    Ok((client, start.elapsed()))
}

/// Durations of each part of the voting process.
#[derive(Debug, Default)]
struct VoteTimings {
    join: StdDuration,
    cast: StdDuration,
    confirm: StdDuration,
}

impl AddAssign for VoteTimings {
    fn add_assign(&mut self, rhs: Self) {
        self.join += rhs.join;
        self.cast += rhs.cast;
        self.confirm += rhs.confirm;
    }
}

impl Div<u32> for VoteTimings {
    type Output = Self;

    fn div(self, rhs: u32) -> Self {
        Self {
            join: self.join / rhs,
            cast: self.cast / rhs,
            confirm: self.confirm / rhs,
        }
    }
}

/// Cast a vote and either confirm or audit as per the `confirm_mode`. The `client` must be
/// pre-authenticated.
fn cast_vote(
    url: &str,
    eid: &str,
    client: &Client,
    confirm_mode: ConfirmMode,
) -> anyhow::Result<VoteTimings> {
    // Join the election.
    let pre_join = Instant::now();
    client
        .post(url!(url, "elections", eid, "join"))
        .json(&json!({}))
        .send()
        .and_then(Response::error_for_status)?;
    let post_join = Instant::now();

    // Cast provisional vote.
    let pre_cast = Instant::now();
    let vote = json!([{
        "question": 1,
        "candidate": CANDIDATES.choose(&mut rand::thread_rng()),
    }]);
    let resp = client
        .post(url!(url, "elections", eid, "votes/cast"))
        .json(&vote)
        .send()
        .and_then(Response::error_for_status)?;
    let post_cast = Instant::now();

    #[derive(Deserialize)]
    struct Receipt {
        ballot_id: u32,
        question_id: u32,
        signature: String,
    }
    let [Receipt {
        question_id,
        ballot_id,
        signature,
    }] = resp.json()?;

    // Confirm (or audit) vote.
    let pre_confirm = Instant::now();
    let confirmation = json!([{
        "ballot_id": ballot_id,
        "question_id": question_id,
        "signature": signature,
    }]);
    let endpoint = if confirm_mode.should_confirm() {
        "votes/confirm"
    } else {
        "votes/audit"
    };
    client
        .post(url!(url, "elections", eid, endpoint))
        .json(&confirmation)
        .send()
        .and_then(Response::error_for_status)?;
    let post_confirm = Instant::now();

    Ok(VoteTimings {
        join: post_join.duration_since(pre_join),
        cast: post_cast.duration_since(pre_cast),
        confirm: post_confirm.duration_since(pre_confirm),
    })
}

/// Run the benchmark.
fn benchmark(
    url: &str,
    eid: &str,
    num_threads: usize,
    confirm_mode: ConfirmMode,
) -> anyhow::Result<()> {
    const ITERATIONS_PER_THREAD: usize = 100;
    let end_val: usize = num_threads * ITERATIONS_PER_THREAD;

    let start = Instant::now();
    thread::scope(|s| {
        let mut threads = Vec::with_capacity(num_threads);

        for start in (0..end_val).step_by(ITERATIONS_PER_THREAD) {
            let t = s.spawn(move || {
                let mut auth_duration = StdDuration::ZERO;
                let mut vote_duration = VoteTimings::default();

                for voter_id in start..(start + ITERATIONS_PER_THREAD) {
                    let (client, auth_dur) = voter_auth(url, voter_id as u32)?;
                    let vote_dur = cast_vote(url, eid, &client, confirm_mode)?;

                    auth_duration += auth_dur;
                    vote_duration += vote_dur;
                }

                let avg_auth_dur = auth_duration / ITERATIONS_PER_THREAD as u32;
                let avg_vote_dur = vote_duration / ITERATIONS_PER_THREAD as u32;
                Ok::<_, anyhow::Error>((avg_auth_dur, avg_vote_dur))
            });
            threads.push(t);
        }

        let mut auth_duration = StdDuration::ZERO;
        let mut vote_duration = VoteTimings::default();
        for t in threads {
            let (auth_dur, vote_dur) = t.join().expect("thread panicked")?;
            auth_duration += auth_dur;
            vote_duration += vote_dur;
        }
        let total_duration = start.elapsed();

        let avg_auth_duration = auth_duration / num_threads as u32;
        let avg_vote_duration = vote_duration / num_threads as u32;
        let avg_total_duration = avg_auth_duration
            + avg_vote_duration.join
            + avg_vote_duration.cast
            + avg_vote_duration.confirm;

        // Theoretical votes per sec is 1/avg_duration * num_threads.
        let votes_per_sec = num_threads as f64 / avg_total_duration.as_secs_f64();
        // Actual votes per sec is total_votes / total_time.
        let actual_votes_per_sec = end_val as f64 / total_duration.as_secs_f64();

        println!("auth: {:?}", avg_auth_duration);
        println!("join: {:?}", avg_vote_duration.join);
        println!("cast: {:?}", avg_vote_duration.cast);
        println!("conf: {:?}", avg_vote_duration.confirm);

        println!("\ntotal: {:?} ({:.2}/s)", avg_total_duration, votes_per_sec);
        println!(
            "actual duration: {} votes in {:?} ({:.2}/s)",
            end_val, total_duration, actual_votes_per_sec
        );

        Ok(())
    })
}

/// Ensure that all dependencies for the verifier are ready.
fn setup_verifier() -> anyhow::Result<()> {
    // Build the binary.
    Command::new("cargo")
        .args([
            "build",
            "--release",
            "--bin",
            "verification-cli",
            "--features",
            "verification",
        ])
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("verification-cli build exited nonzero"))?;

    Ok(())
}

/// Return `Ok(())` if the election integrity can be successfully verified.
fn verify(url: &str, eid: &str) -> anyhow::Result<()> {
    // Hack the DB to mark the election as finished, so we can get the totals out.
    let end_time = Utc::now() - Duration::minutes(1);
    let mongosh_cmd = format!(
        "db.elections.updateOne(\
            {{_id: {}}}, {{$set: {{end_time: ISODate('{}')}}}})",
        eid,
        end_time.to_rfc3339()
    );
    let db_uri = env::var("ROCKET_DB_URI")?;

    Command::new("mongosh")
        .args([
            &db_uri,
            "--quiet",
            "--eval",
            "use dreip",
            "--eval",
            &mongosh_cmd,
        ])
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("failed to modify DB"))?;

    // Dump the election results.
    let client = Client::new();
    let dump = client
        .get(url!(url, "elections", eid, "1/dump"))
        .send()
        .and_then(Response::error_for_status)?
        .bytes()?;

    // Sanity-check that we actually got the totals.
    let obj: HashMap<String, IgnoredAny> = serde_json::from_slice(&dump)?;
    if !obj.contains_key("totals") {
        return Err(anyhow!("election dump missing totals"));
    }

    // Dump the dump to a file and run the verifier on it.
    let mut f = NamedTempFile::new()?;
    f.write_all(&dump)?;
    f.flush()?;

    Command::new("./target/release/verification-cli")
        .arg(f.path())
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("verification failed"))?;

    Ok(())
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let url = args.remote.as_deref().unwrap_or(LOCAL_URL);

    // Pre-build the verification CLI if requested.
    if args.verify {
        setup_verifier()?;
    }

    // If we're not connecting remotely, bring up a local server.
    let mut proc: Option<Child> = None;
    if args.remote.is_none() {
        setup_deps(args.reuse_db)?;
        let logfile = match args.logfile {
            Some(path) => Stdio::from(File::open(path)?),
            None => {
                if args.quiet {
                    Stdio::null()
                } else {
                    Stdio::inherit()
                }
            }
        };
        proc = Some(launch_server(logfile)?);
    }

    // Use a closure to ensure the cleanup below runs.
    let result = (|| {
        // Run the benchmark.
        let eid = setup_election(url)?;
        benchmark(url, &eid, args.threads, args.confirm_mode)?;

        // Verify if requested.
        if args.verify {
            verify(url, &eid)?;
        }

        Ok(())
    })();

    // Kill the server.
    if let Some(p) = proc.as_mut() {
        terminate_child(p)?;
        p.wait()?;
    }

    result
}

fn main() {
    if let Err(e) = run() {
        eprintln!("FATAL: {}", e);
        process::exit(1);
    }
}

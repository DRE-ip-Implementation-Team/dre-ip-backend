use log::{error, info, LevelFilter};
use mongodb::error::Error as DbError;
use rocket::Error as RocketError;
use thiserror::Error;

/// Errors that are critical to the entire server.
#[derive(Debug, Error)]
enum Error {
    #[error("Failed to contact database during launch: {0}")]
    DbLaunchError(#[from] DbError),
    #[error(transparent)]
    RocketError(#[from] RocketError),
}

async fn run() -> Result<(), Error> {
    info!("Configuring server...");
    let rocket = dreip_backend::build().await?.ignite().await?;
    info!("...server configured!");
    let protocol = rocket
        .config()
        .tls_enabled()
        .then(|| "https")
        .unwrap_or("http");
    let ip = &rocket.config().address;
    let port = &rocket.config().port;
    info!("Server launched on {protocol}://{ip}:{port}");
    // Disable rocket logging from now on.
    log4rs_dynamic_filters::DynamicLevelFilter::set("rocket", LevelFilter::Off);
    let _ = rocket.launch().await?;
    Ok(())
}

#[rocket::main]
async fn main() {
    // Set up logging.
    log4rs::init_file("log4rs.yaml", log4rs_dynamic_filters::default_deserializers())
        .expect("Failed to initialise logging");
    info!("Initialised logging");

    // Launch server.
    if let Err(err) = run().await {
        error!("{err}");
        error!("Critical failure, shutting down");
        std::process::exit(1)
    }
}

use log::{error, info, LevelFilter};
use rocket::Error as RocketError;

async fn run() -> Result<(), RocketError> {
    info!("Configuring server...");
    let rocket = dreip_backend::build().ignite().await?;
    info!("...server configured!");
    // Disable rocket logging from now on.
    log4rs_dynamic_filters::DynamicLevelFilter::set("rocket", LevelFilter::Off);
    let _ = rocket.launch().await?;
    Ok(())
}

#[rocket::main]
async fn main() {
    // Set up logging.
    log4rs::init_file(
        "log4rs.yaml",
        log4rs_dynamic_filters::default_deserializers(),
    )
    .expect("Failed to initialise logging");
    info!("Initialised logging");

    // Launch server.
    if let Err(err) = run().await {
        error!("{err}");
        error!("Critical failure, shutting down");
        std::process::exit(1)
    }
}

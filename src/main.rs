#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> _ {
    dreip_backend::build().await
}

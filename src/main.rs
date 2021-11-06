#[macro_use] extern crate rocket;

#[launch]
fn rocket() -> _ {
    dreip_backend::build()
}

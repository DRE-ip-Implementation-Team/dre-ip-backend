use rocket::Route;

pub mod election_management;
pub mod information;
pub mod login;
pub mod voting;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(information::routes());
    routes.extend(login::routes());
    routes.extend(election_management::routes());
    routes.extend(voting::routes());
    routes
}

use rocket::Route;

mod election_management;
mod information;
mod login;
mod voting;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(information::routes());
    routes.extend(login::routes());
    routes.extend(election_management::routes());
    routes.extend(voting::routes());
    routes
}

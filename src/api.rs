use rocket::Route;

mod information;
mod login;
mod election_management;
mod voting;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.append(&mut information::routes());
    routes.append(&mut login::routes());
    routes.append(&mut election_management::routes());
    routes.append(&mut voting::routes());
    return routes;
}

use rocket::{
    form::{Form, Strict},
    Route, State,
};

use crate::{
    error::Result,
    model::{
        admin::{Admin, Credentials, PutAdmins},
        auth::token::Token,
    },
};

pub fn routes() -> Vec<Route> {
    routes![create_admin]
}

#[post("/admins", data = "<credentials>")]
async fn create_admin(
    _token: Token<Admin>,
    credentials: Form<Strict<Credentials<'_>>>,
    admins: &State<PutAdmins>,
) -> Result<()> {
    let admin = credentials.into_admin();
    admins.insert_one(admin, None).await?;
    Ok(())
}
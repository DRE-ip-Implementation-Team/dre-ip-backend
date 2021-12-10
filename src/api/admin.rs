use rocket::{
    form::{Form, Strict},
    Route, State,
};

use crate::{
    error::Result,
    model::admin::{Credentials, PutAdmins},
};

pub fn routes() -> Vec<Route> {
    routes![create_admin]
}

#[post("/admin", data = "<credentials>")]
async fn create_admin(
    credentials: Form<Strict<Credentials<'_>>>,
    admins: &State<PutAdmins>,
) -> Result<()> {
    let admin = credentials.into_admin()?;
    admins.insert_one(admin, None).await?;
    Ok(())
}

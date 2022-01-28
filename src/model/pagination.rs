use rocket::form::{self, DataField, Errors, FromForm, FromFormField, ValueField};
use serde::Serialize;

#[derive(UriDisplayQuery)]
pub struct Pagination {
    page_num: u32,
    page_size: u32,
}

impl Pagination {
    pub fn new(page_num: u32, page_size: u32) -> Self {
        Self {
            page_num,
            page_size,
        }
    }

    pub fn skip(&self) -> u32 {
        (self.page_num - 1) * self.page_size
    }

    pub fn page_size(&self) -> u32 {
        self.page_size
    }

    pub fn into_metadata(self, total: usize) -> Metadata {
        Metadata {
            page_num: self.page_num,
            page_size: self.page_size,
            total,
        }
    }
}

pub struct Context<'f> {
    page_num: u32,
    page_size: u32,
    errors: Errors<'f>,
}

#[rocket::async_trait]
impl<'r> FromForm<'r> for Pagination {
    type Context = Context<'r>;

    fn init(_opts: form::Options) -> Self::Context {
        Context {
            page_num: 1,
            page_size: 50,
            errors: Errors::default(),
        }
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
        if field.name == "page_num" {
            match u32::from_value(field) {
                Ok(page_num) => ctxt.page_num = page_num,
                Err(errs) => ctxt.errors.extend(errs),
            }
        } else if field.name == "page_size" {
            match u32::from_value(field) {
                Ok(page_size) => ctxt.page_size = page_size,
                Err(errs) => ctxt.errors.extend(errs),
            }
        }
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) {
        if field.name == "page_num" {
            match u32::from_data(field).await {
                Ok(page_num) => ctxt.page_num = page_num,
                Err(errs) => ctxt.errors.extend(errs),
            }
        } else if field.name == "page_size" {
            match u32::from_data(field).await {
                Ok(page_size) => ctxt.page_size = page_size,
                Err(errs) => ctxt.errors.extend(errs),
            }
        }
    }

    fn finalize(ctxt: Self::Context) -> form::Result<'r, Self> {
        if ctxt.errors.is_empty() {
            Ok(Self {
                page_num: ctxt.page_num,
                page_size: ctxt.page_size,
            })
        } else {
            Err(ctxt.errors)
        }
    }
}

#[derive(Serialize)]
pub struct Metadata {
    page_num: u32,
    page_size: u32,
    total: usize,
}

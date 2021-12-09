use rocket::form::{self, DataField, Errors, FromForm, FromFormField, ValueField};
use serde::Serialize;

pub struct Pagination {
    page_num: usize,
    page_size: usize,
}

impl Pagination {
    pub fn page_num(&self) -> usize {
        self.page_num
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    pub fn skip(&self) -> u64 {
        ((self.page_num - 1) * self.page_size) as u64
    }

    pub fn result(self, total: usize) -> PaginationResult {
        PaginationResult {
            page_num: self.page_num,
            page_size: self.page_size,
            total,
        }
    }
}

pub struct PaginationContext<'f> {
    page_num: usize,
    page_size: usize,
    errors: Errors<'f>,
}

#[rocket::async_trait]
impl<'r> FromForm<'r> for Pagination {
    type Context = PaginationContext<'r>;

    fn init(_opts: form::Options) -> Self::Context {
        PaginationContext {
            page_num: 1,
            page_size: 50,
            errors: Errors::default(),
        }
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'r>) {
        if field.name == "page_num" {
            match usize::from_value(field) {
                Ok(page_num) => ctxt.page_num = page_num,
                Err(errs) => ctxt.errors.extend(errs),
            }
        } else if field.name == "page_size" {
            match usize::from_value(field) {
                Ok(page_size) => ctxt.page_size = page_size,
                Err(errs) => ctxt.errors.extend(errs),
            }
        }
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'r, '_>) {
        if field.name == "page_num" {
            match usize::from_data(field).await {
                Ok(page_num) => ctxt.page_num = page_num,
                Err(errs) => ctxt.errors.extend(errs),
            }
        } else if field.name == "page_size" {
            match usize::from_data(field).await {
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
pub struct PaginationResult {
    page_num: usize,
    page_size: usize,
    total: usize,
}

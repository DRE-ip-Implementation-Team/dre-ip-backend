use rocket::form::{self, DataField, Errors, FromForm, FromFormField, ValueField};
use serde::{Deserialize, Serialize};

/// Max page size of a single paginated response.
pub const MAX_PAGE_SIZE: u32 = 100;

/// Pagination request from a client - which page do they want, and how big?
#[derive(Debug, UriDisplayQuery)]
pub struct PaginationRequest {
    /// Which page is this?
    pub page_num: u32,
    /// How big is each page?
    pub page_size: u32,
}

impl PaginationRequest {
    /// Calculate how many elements to skip before the start of this page.
    pub fn skip(&self) -> u32 {
        (self.page_num - 1) * self.page_size
    }

    /// Get the page size.
    pub fn page_size(&self) -> u32 {
        std::cmp::min(self.page_size, MAX_PAGE_SIZE)
    }

    /// Convert into a response with the given total number of items.
    pub fn to_response(&self, total: u64) -> PaginationResponse {
        PaginationResponse {
            page_num: self.page_num,
            page_size: self.page_size(),
            total,
        }
    }

    /// Convert into a response with the given total and items.
    pub fn to_paginated<T>(&self, total: u64, items: Vec<T>) -> Paginated<T> {
        Paginated {
            items,
            pagination: self.to_response(total),
        }
    }
}

/// Context struct for parsing from requests.
pub struct Context<'f> {
    page_num: u32,
    page_size: u32,
    errors: Errors<'f>,
}

#[rocket::async_trait]
impl<'r> FromForm<'r> for PaginationRequest {
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

/// Pagination response to a client - which page did you actually get, how big
/// is it actually, and how many items are there in total?
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationResponse {
    /// Which page is this?
    pub page_num: u32,
    /// How big is each page?
    pub page_size: u32,
    /// How many items are there in total?
    pub total: u64,
}

/// A paginated vector of T, with pagination metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub pagination: PaginationResponse,
}
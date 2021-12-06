use rocket::{
    http::Status,
    request::{self, FromRequest, Request},
};
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Pagination {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let page_num = if let Ok(page_num) = req.query_value::<usize>("page_num").unwrap_or(Ok(1)) {
            page_num
        } else {
            return request::Outcome::Failure((Status::BadRequest, ()));
        };
        let page_size =
            if let Ok(page_size) = req.query_value::<usize>("page_size").unwrap_or(Ok(50)) {
                page_size
            } else {
                return request::Outcome::Failure((Status::BadRequest, ()));
            };
        request::Outcome::Success(Self {
            page_num,
            page_size,
        })
    }
}

#[derive(Serialize)]
pub struct PaginationResult {
    page_num: usize,
    page_size: usize,
    total: usize,
}

use std::io::{Read, Cursor};

use rocket::http::ContentType;
use rocket::{http::Status, response, Response, Request};
use rocket::response::Responder;
use serde_json::json;

#[derive(serde::Serialize)]
pub struct Error {
    status: Status,
    message: String,
    solution: String
}

impl Error {
    pub fn new(status: Status, message: String, solution: String) -> Self {
        Self { status, message, solution }
    }

    pub fn build<'r>(&self) -> Response<'r> {
        let body = serde_json::to_string(&self).unwrap_or("{}".to_string());
        Response::build()
            .streamed_body(Cursor::new(body))
            .header(ContentType::JSON)
            .status(self.status)
            .finalize()
    }
}

impl Read for Error {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }
}

impl<'r> Responder<'r, 'r> for Error {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        let body = serde_json::to_string(&self).unwrap_or("{}".to_string());
        Response::build()
            .streamed_body(Cursor::new(body))
            .header(ContentType::JSON)
            .status(self.status)
            .ok()
    }
}
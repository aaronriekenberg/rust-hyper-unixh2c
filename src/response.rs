use bytes::Bytes;

use http_body_util::combinators::BoxBody;

use hyper::http::HeaderValue;

use std::convert::Infallible;

#[derive(Clone, Copy, Debug)]
pub enum CacheControl {
    NoCache,
    // Cache { max_age_seconds: u32 },
}

impl CacheControl {
    pub fn header_value(&self) -> HeaderValue {
        static NO_CACHE_VALUE: HeaderValue = HeaderValue::from_static("public, no-cache");

        match self {
            CacheControl::NoCache => NO_CACHE_VALUE.clone(),
            // CacheControl::Cache { max_age_seconds } => {
            //    format!("public, max-age={}", max_age_seconds)
            // }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ResponseBodyError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<Infallible> for ResponseBodyError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub type ResponseBody = BoxBody<Bytes, ResponseBodyError>;

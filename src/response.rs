use bytes::Bytes;

use http_body_util::combinators::BoxBody;

use hyper::http::HeaderValue;

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

pub type ResponseBody = BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>;

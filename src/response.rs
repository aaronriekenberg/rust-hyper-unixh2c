use bytes::Bytes;

use http_body_util::combinators::BoxBody;

#[derive(Clone, Copy, Debug)]
pub enum CacheControl {
    NoCache,
    // Cache { max_age_seconds: u32 },
}

impl CacheControl {
    pub fn header_value(&self) -> String {
        match self {
            CacheControl::NoCache => "public, no-cache".to_owned(),
            // CacheControl::Cache { max_age_seconds } => {
            //     format!("public, max-age={}", max_age_seconds)
            // }
        }
    }
}

pub type ResponseBody = BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>;

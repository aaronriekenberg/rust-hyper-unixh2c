use bytes::Bytes;

use http_body_util::combinators::BoxBody;

pub type ResponseBody = BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>;

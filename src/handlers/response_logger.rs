use async_trait::async_trait;

use hyper::http::Response;

use tracing::{info, warn};

use crate::handlers::{HttpRequest, RequestHandler, ResponseBody};

pub struct ResponseLogger {
    next_handler: Box<dyn RequestHandler>,
}

impl ResponseLogger {
    pub fn new(next_handler: Box<dyn RequestHandler>) -> Self {
        Self { next_handler }
    }
}
#[async_trait]
impl RequestHandler for ResponseLogger {
    async fn handle(&self, request: &HttpRequest) -> Response<ResponseBody> {
        let response = self.next_handler.handle(request).await;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();

            if response.status().is_informational()
                || response.status().is_redirection()
                || response.status().is_client_error()
            {
                info!("status_code = {}", status_code);
            } else {
                warn!("status_code = {}", status_code);
            }
        }

        response
    }
}

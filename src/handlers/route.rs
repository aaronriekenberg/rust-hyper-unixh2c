use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;

use async_trait::async_trait;

use hyper::{http::Method, Body, Response};

use crate::handlers::{utils::build_status_code_response, HttpRequest, RequestHandler};

pub struct RouteInfo {
    pub method: Method,
    pub path_suffix: PathBuf,
    pub handler: Box<dyn RequestHandler>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct RouteKey {
    method: Method,
    path: String,
}

impl From<&HttpRequest> for RouteKey {
    fn from(http_request: &HttpRequest) -> Self {
        Self {
            method: http_request.hyper_request().method().clone(),
            path: http_request.hyper_request().uri().path().to_owned(),
        }
    }
}

pub struct Router {
    route_key_to_handler: HashMap<RouteKey, Box<dyn RequestHandler>>,
}

impl Router {
    pub fn new(routes: Vec<RouteInfo>) -> anyhow::Result<Self> {
        let mut router = Self {
            route_key_to_handler: HashMap::with_capacity(routes.len()),
        };

        let context_configuration = crate::config::instance().context_configuration();

        for route in routes {
            let uri_pathbuf =
                PathBuf::from(context_configuration.context()).join(route.path_suffix);

            let path = uri_pathbuf.to_str().with_context(|| {
                format!(
                    "Router::new error: route path contains invalid UTF-8 uri_pathbuf = '{:?}'",
                    uri_pathbuf,
                )
            })?;

            let key = RouteKey {
                method: route.method,
                path: path.to_owned(),
            };

            if router
                .route_key_to_handler
                .insert(key.clone(), route.handler)
                .is_some()
            {
                anyhow::bail!("Router::new error: collision in router key '{:?}'", key);
            }
        }
        Ok(router)
    }
}

#[async_trait]
impl RequestHandler for Router {
    async fn handle(&self, request: HttpRequest) -> Response<Body> {
        let route_key = RouteKey::from(&request);

        match self.route_key_to_handler.get(&route_key) {
            Some(handler) => handler.handle(request).await,
            None => build_status_code_response(hyper::http::StatusCode::NOT_FOUND),
        }
    }
}

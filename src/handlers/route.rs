use anyhow::Context;

use async_trait::async_trait;

use bytes::Bytes;

use http_body_util::combinators::BoxBody;

use hyper::http::{Method, Response};

use tracing::debug;

use std::{
    borrow::Cow,
    collections::HashMap,
    convert::Infallible,
    path::{Path, PathBuf},
};

use crate::handlers::{HttpRequest, RequestHandler};

pub struct RouteInfo {
    pub method: &'static Method,
    pub path_suffix: PathBuf,
    pub handler: Box<dyn RequestHandler>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct RouteKey<'a> {
    method: &'a Method,
    path: Cow<'a, str>,
}

impl<'a> From<&'a HttpRequest> for RouteKey<'a> {
    fn from(http_request: &'a HttpRequest) -> Self {
        Self {
            method: http_request.hyper_request().method(),
            path: Cow::from(http_request.hyper_request().uri().path()),
        }
    }
}

pub struct Router {
    route_key_to_handler: HashMap<RouteKey<'static>, Box<dyn RequestHandler>>,
    default_route: Box<dyn RequestHandler>,
}

impl Router {
    pub fn new(
        routes: Vec<RouteInfo>,
        default_route: Box<dyn RequestHandler>,
    ) -> anyhow::Result<Self> {
        let mut router = Self {
            route_key_to_handler: HashMap::with_capacity(routes.len()),
            default_route,
        };

        let context_path = Path::new(crate::config::instance().context_configuration().context());

        for route in routes {
            let route_key = Self::build_route_key(context_path, &route)?;

            if router
                .route_key_to_handler
                .insert(route_key.clone(), route.handler)
                .is_some()
            {
                anyhow::bail!(
                    "Router::new error: collision in router key = {:?}",
                    route_key,
                );
            }
        }
        Ok(router)
    }

    fn build_route_key(
        context_path: &Path,
        route: &RouteInfo,
    ) -> anyhow::Result<RouteKey<'static>> {
        let path = context_path.join(&route.path_suffix);

        let path = path
            .to_str()
            .with_context(|| {
                format!(
                    "Router::build_route_key error: uri_pathbuf.to_str error uri_pathbuf = '{:?}'",
                    path,
                )
            })?
            .to_owned();

        Ok(RouteKey {
            method: route.method,
            path: Cow::from(path),
        })
    }
}

#[async_trait]
impl RequestHandler for Router {
    async fn handle(&self, request: &HttpRequest) -> Response<BoxBody<Bytes, std::io::Error>> {
        debug!("begin handle");

        let handler_option = self.route_key_to_handler.get(&RouteKey::from(request));

        let response = match handler_option {
            Some(handler) => handler.handle(request).await,
            None => self.default_route.handle(request).await,
        };

        debug!("end handle");
        response
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_route_key_equality() {
        assert_eq!(
            RouteKey {
                method: &Method::GET,
                path: Cow::Borrowed("/test"),
            },
            RouteKey {
                method: &Method::GET,
                path: Cow::Owned("/test".to_owned()),
            }
        );

        assert_ne!(
            RouteKey {
                method: &Method::GET,
                path: Cow::Borrowed("/test"),
            },
            RouteKey {
                method: &Method::PUT,
                path: Cow::Owned("/test".to_owned()),
            }
        );

        assert_ne!(
            RouteKey {
                method: &Method::GET,
                path: Cow::Borrowed("/nottest"),
            },
            RouteKey {
                method: &Method::GET,
                path: Cow::Owned("/test".to_owned()),
            }
        );
    }

    #[test]
    fn test_route_key_hash() {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };

        let key1 = RouteKey {
            method: &Method::GET,
            path: Cow::Borrowed("/test"),
        };

        let key2 = RouteKey {
            method: &Method::GET,
            path: Cow::Owned("/test".to_owned()),
        };

        let mut s = DefaultHasher::new();
        key1.hash(&mut s);
        let key1_hash = s.finish();

        let mut s = DefaultHasher::new();
        key2.hash(&mut s);
        let key2_hash = s.finish();

        assert_eq!(key1_hash, key2_hash);
    }
}

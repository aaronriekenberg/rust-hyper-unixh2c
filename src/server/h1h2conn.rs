use hyper::{
    rt::bounds::Http2ConnExec,
    server::conn::{
        http1::Connection as HyperHTTP1Connection, http2::Connection as HyperHTTP2Connection,
    },
};

use pin_project::pin_project;

use tokio::pin;

use std::pin::Pin;

use crate::{
    response::ResponseBody,
    server::utils::{HyperHttpService, HyperReadWrite},
};

#[pin_project(project = HyperH1OrH2ConnectionProj)]
pub enum HyperH1OrH2Connection<I, S, E>
where
    I: HyperReadWrite,
    S: HyperHttpService,
    E: Http2ConnExec<S::Future, ResponseBody>,
{
    H1(#[pin] HyperHTTP1Connection<I, S>),
    H2(#[pin] HyperHTTP2Connection<I, S, E>),
}

impl<I, S, E> std::future::Future for HyperH1OrH2Connection<I, S, E>
where
    I: HyperReadWrite,
    S: HyperHttpService,
    E: Http2ConnExec<S::Future, ResponseBody>,
{
    type Output = hyper::Result<()>;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project() {
            HyperH1OrH2ConnectionProj::H1(h1_conn) => h1_conn.poll(cx),
            HyperH1OrH2ConnectionProj::H2(h2_conn) => h2_conn.poll(cx),
        }
    }
}

impl<I, S, E> HyperH1OrH2Connection<I, S, E>
where
    I: HyperReadWrite,
    S: HyperHttpService,
    E: Http2ConnExec<S::Future, ResponseBody>,
{
    pub fn graceful_shutdown(self: Pin<&mut Self>) {
        match self.project() {
            HyperH1OrH2ConnectionProj::H1(h1_conn) => h1_conn.graceful_shutdown(),
            HyperH1OrH2ConnectionProj::H2(h2_conn) => h2_conn.graceful_shutdown(),
        }
    }
}

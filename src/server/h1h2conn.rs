use hyper::{
    server::conn::http1::Connection as HyperHTTP1Connection,
    server::conn::http2::Connection as HyperHTTP2Connection,
};

use hyper_util::rt::TokioIo;

use pin_project::pin_project;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    pin,
};

use std::{convert::Infallible, pin::Pin};

use crate::response::ResponseBody;

#[pin_project(project = HyperH1OrH2ConnectionProj)]
pub enum HyperH1OrH2Connection<I, S, E>
where
    I: AsyncRead + AsyncWrite + Unpin + 'static,
    S: hyper::service::HttpService<
        hyper::body::Incoming,
        ResBody = ResponseBody,
        Error = Infallible,
    >,
    E: hyper::rt::bounds::Http2ConnExec<S::Future, ResponseBody>,
{
    H1(#[pin] HyperHTTP1Connection<TokioIo<I>, S>),
    H2(#[pin] HyperHTTP2Connection<TokioIo<I>, S, E>),
}

impl<I, S, E> std::future::Future for HyperH1OrH2Connection<I, S, E>
where
    I: AsyncRead + AsyncWrite + Unpin + 'static,
    S: hyper::service::HttpService<
        hyper::body::Incoming,
        ResBody = ResponseBody,
        Error = Infallible,
    >,
    E: hyper::rt::bounds::Http2ConnExec<S::Future, ResponseBody>,
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
    I: AsyncRead + AsyncWrite + Unpin + 'static,
    S: hyper::service::HttpService<
        hyper::body::Incoming,
        ResBody = ResponseBody,
        Error = Infallible,
    >,
    E: hyper::rt::bounds::Http2ConnExec<S::Future, ResponseBody>,
{
    pub fn graceful_shutdown(self: Pin<&mut Self>) {
        match self.project() {
            HyperH1OrH2ConnectionProj::H1(h1_conn) => h1_conn.graceful_shutdown(),
            HyperH1OrH2ConnectionProj::H2(h2_conn) => h2_conn.graceful_shutdown(),
        }
    }
}

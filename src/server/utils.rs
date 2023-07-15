use hyper::service::HttpService;

use std::convert::Infallible;

use crate::response::ResponseBody;

pub trait HyperReadWrite: hyper::rt::Read + hyper::rt::Write + Unpin + 'static {}

impl<T> HyperReadWrite for T where T: hyper::rt::Read + hyper::rt::Write + Unpin + 'static {}

pub trait HyperHttpService:
    HttpService<hyper::body::Incoming, ResBody = ResponseBody, Error = Infallible>
{
}

impl<T> HyperHttpService for T where
    T: HttpService<hyper::body::Incoming, ResBody = ResponseBody, Error = Infallible>
{
}

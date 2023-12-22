pub trait HyperReadWrite: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static {}

impl<T> HyperReadWrite for T where T: hyper::rt::Read + hyper::rt::Write + Send + Unpin + 'static {}

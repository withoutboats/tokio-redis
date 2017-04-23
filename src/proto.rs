use std::io;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_proto::pipeline::ClientProto;

use cmd::Cmd;
use transport::RedisTransport;
use types::Value;

pub struct RedisProto;

impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for RedisProto {
    type Request = Cmd;
    type Response = Value;
    type Transport = RedisTransport<T>;
    type BindTransport = io::Result<Self::Transport>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(RedisTransport::new(io))
    }
}


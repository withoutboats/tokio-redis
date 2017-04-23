#![allow(dead_code)]

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;

#[macro_use]
extern crate log;

mod cmd;
mod macros;
mod parser;
mod proto;
mod transport;
mod types;

use std::io;
use std::net::SocketAddr;

use futures::Future;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_proto::{TcpClient, BoundTcpClient};
use tokio_proto::pipeline::{Pipeline, ClientService};
use tokio_service::{NewService, Service};

use proto::RedisProto;

pub use cmd::Cmd;

pub use types::{
    /* low level values */
    Value,

    /* error and result types */
    RedisError as Error,

    /* error kinds */
    ErrorKind,

    RedisResult as Result,

    /* conversion traits */
    FromRedisValue,
    ToRedisArgs,
};

pub type Response = Box<Future<Item = Value, Error = io::Error>>;

pub struct Redis {
    client: BoundTcpClient<Pipeline, RedisProto>,
}

impl Redis {
    pub fn new(addr: SocketAddr, handle: Handle) -> Redis {
        Redis {
            client: TcpClient::new(RedisProto).bind(addr, handle),
        }
    }
}

impl NewService for Redis {
    type Request = Cmd;
    type Response = Value;
    type Error = io::Error;
    type Instance = Client;
    type Future = Box<Future<Item = Client, Error = io::Error>>;

    fn new_service(&self) -> Self::Future {
        Box::new(self.client.new_service().map(|client| Client { inner: client }))
    }
}

pub struct Client {
    inner: ClientService<TcpStream, RedisProto>,
}

impl Client {
    /// Get the value of a key.  If key is a vec this becomes an `MGET`.
    pub fn get<K: ToRedisArgs>(&mut self, key: K) -> Response {
        let mut cmd = Cmd::new();
        cmd.arg(if key.is_single_arg() { "GET" } else { "MGET" }).arg(key);

        self.call(cmd)
    }

    /// Set the string value of a key.
    pub fn set<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V) -> Response {
        let mut cmd = Cmd::new();
        cmd.arg("SET").arg(key).arg(value);

        self.call(cmd)
    }

    pub fn keys<K: ToRedisArgs>(&mut self, key: K) -> Response {
        let mut cmd = Cmd::new();
        cmd.arg("KEYS").arg(key);

        self.call(cmd)
    }

    pub fn set_ex<K: ToRedisArgs, V: ToRedisArgs>(&mut self, key: K, value: V, seconds: usize) -> Response {
        let mut cmd = Cmd::new();
        cmd.arg("SETEX").arg(key).arg(seconds).arg(value);

        self.call(cmd)
    }
}

impl Service for Client {
    type Request = Cmd;
    type Response = Value;
    type Error = io::Error;
    type Future = Response;

    fn call(&self, req: Cmd) -> Response {
        Box::new(self.inner.call(req))
    }
}

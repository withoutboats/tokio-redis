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

    Okay,
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

type ClientResponse<'a, V> = Box<Future<Item = V, Error = Error> + 'a>;

impl Client {
    /// Get the value of a key.  If key is a vec this becomes an `MGET`.
    pub fn get<'a, K: ToRedisArgs, V: FromRedisValue + 'a>(&self, key: K) -> ClientResponse<'a, V> {
        let mut cmd = Cmd::new();
        cmd.arg(if key.is_single_arg() { "GET" } else { "MGET" }).arg(key);

        Box::new(self.call(cmd).then(|res| match res {
            Ok(v)   => V::from_redis_value(&v),
            Err(e)  => Err(e.into()),
        }))
    }

    /// Set the string value of a key.
    pub fn set<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> ClientResponse<'static, Option<Okay>> {
        let mut cmd = Cmd::new();
        cmd.arg("SET").arg(key).arg(value);

        Box::new(self.call(cmd).then(|res| match res {
            Ok(v)   => Option::from_redis_value(&v),
            Err(e)  => Err(e.into()),
        }))
    }

    pub fn keys<'a, K: ToRedisArgs, V: FromRedisValue + 'a>(&self, key: K) -> ClientResponse<'a, Vec<V>> {
        let mut cmd = Cmd::new();
        cmd.arg("KEYS").arg(key);

        Box::new(self.call(cmd).then(|res| match res {
            Ok(v)   => Vec::from_redis_value(&v),
            Err(e)  => Err(e.into()),
        }))
    }

    pub fn set_ex<K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V, seconds: usize) -> ClientResponse<'static, Option<Okay>> {
        let mut cmd = Cmd::new();
        cmd.arg("SETEX").arg(key).arg(seconds).arg(value);

        Box::new(self.call(cmd).then(|res| match res {
            Ok(v)   => Option::from_redis_value(&v),
            Err(e)  => Err(e.into()),
        }))
    }

    /// Set the string value of a key if it does not exist.
    pub fn setnx<'a, K: ToRedisArgs, V: ToRedisArgs>(&self, key: K, value: V) -> ClientResponse<'a, Option<Okay>> {
        let mut cmd = Cmd::new();
        cmd.arg("SETNX").arg(key).arg(value);

        Box::new(self.call(cmd).then(|res| match res {
            Ok(v)   => Option::from_redis_value(&v),
            Err(e)  => Err(e.into()),
        }))
    }

    /// Increment the value of an integer key, from 0 if it doesn't exist.
    pub fn incr<'a, K: ToRedisArgs, V: FromRedisValue + 'a>(&self, key: K) -> ClientResponse<'a, V> {
        let mut cmd = Cmd::new();
        cmd.arg("INCR").arg(key);

        Box::new(self.call(cmd).then(|res| match res {
            Ok(v)   => V::from_redis_value(&v),
            Err(e)  => Err(e.into()),
        }))
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

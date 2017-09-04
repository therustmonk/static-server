extern crate futures;
extern crate futures_cpupool;
extern crate tokio_io;
extern crate bytes;
extern crate hyper;

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use futures::{Future, Stream, Sink};
use futures::sync::mpsc::{channel, Sender};
use futures_cpupool::CpuPool;
use tokio_io::AsyncRead;
use tokio_io::codec::{FramedRead, Decoder};
use bytes::BytesMut;
use hyper::{Chunk, Body, Get, StatusCode};
use hyper::server::{Http, Service, NewService, Request, Response};

pub trait Provider: Send + Sync + 'static {
    fn find_file<'a>(&self, path: &'a str) -> Option<Box<AsyncRead + Send>>;
}

pub struct StaticService<T: Provider> {
    provider: Arc<T>,
    sender: Sender<Job>,
}

impl<T: Provider> Service for StaticService<T> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        if req.method() == &Get {
            let path = req.path();
            if let Some(reader) = self.provider.find_file(path) {
                let (tx, body) = Body::pair();
                let fut = self.sender.clone().send((tx, reader))
                    .map_err(|_| other("can't send task").into())
                    .map(|_| {
                        Response::new().with_body(body)
                    });
                Box::new(fut)
            } else {
                let fut = futures::future::ok(
                    Response::new().with_status(StatusCode::NotFound)
                );
                Box::new(fut)
            }
        } else {
            let fut = futures::future::ok(
                Response::new().with_status(StatusCode::MethodNotAllowed)
            );
            Box::new(fut)
        }
    }
}

type Job = (Sender<hyper::Result<Chunk>>, Box<AsyncRead + Send>);

pub struct StaticNewService<T: Provider> {
    provider: Arc<T>,
    sender: Sender<Job>,
}

impl<T: Provider> NewService for StaticNewService<T> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = StaticService<T>;

    fn new_service(&self) -> Result<Self::Instance, io::Error> {
        Ok(StaticService {
            provider: self.provider.clone(),
            sender: self.sender.clone(),
        })
    }
}

pub fn serve<T: Provider>(addr: &SocketAddr, provider: T) {
    let pool = CpuPool::new_num_cpus();
    let (tx, rx) = channel(10);
    let new_service = StaticNewService {
        provider: Arc::new(provider),
        sender: tx,
    };
    if let Ok(server) = Http::new().bind(&addr, new_service) {
        let handle = server.handle();
        let routine = rx.fold(pool, |pool, (sender_chunk, async_read)| {
            let decoder = ChunkDecoder::new(1024);
            let framed = FramedRead::new(async_read, decoder);
            let job = framed.fold(sender_chunk, |sender, chunk| {
                sender.send(Ok(chunk))
                    .map_err(|_| other("can't send chunk to the channel"))
            });
            pool.spawn(job).forget();
            Ok(pool)
        }).map(|_| ());
        handle.spawn(routine);
        server.run().expect("Can't run a static server!");
    }
}

fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}

struct ChunkDecoder {
    length: usize,
}

impl ChunkDecoder {
    fn new(length: usize) -> Self {
        ChunkDecoder {
            length,
        }
    }
}

impl Decoder for ChunkDecoder {
    type Item = Chunk;
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut
    ) -> Result<Option<Self::Item>, Self::Error> {
        Ok(if src.len() >= self.length {
            let bs = src.split_to(self.length);
            Some(bs.to_vec().into())
        } else {
            None
        })
    }
}

extern crate futures;
extern crate futures_cpupool;
extern crate tokio_io;
extern crate bytes;
extern crate hyper;

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use futures::{Future, Stream, Sink};
use futures::sync::{oneshot, mpsc};
use futures_cpupool::CpuPool;
use tokio_io::AsyncRead;
use tokio_io::codec::{FramedRead, Decoder};
use bytes::BytesMut;
use hyper::{Chunk, Body, Get, StatusCode};
use hyper::server::{Http, Service, NewService, Request, Response};


type SendFile = Box<AsyncRead + Send>;

enum InMsg {
    Register(String, SendFile),
    Find(String, oneshot::Sender<Option<SendFile>>),
}

pub struct OnceProvider {
    worker: mpsc::Sender<InMsg>,
}

impl OnceProvider {
    pub fn new() -> (Self, Box<Future<Item=(), Error=()>>) {
        let (tx, rx) = mpsc::channel(10);
        let provider = OnceProvider {
            worker: tx,
        };
        let map: HashMap<String, SendFile> = HashMap::new();
        let fut = rx.fold(map, |mut map, msg| {
            match msg {
                InMsg::Register(path, file) => {
                    map.insert(path, file);
                    Ok(map)
                },
                InMsg::Find(path, sender) => {
                    let opt = map.remove(&path);
                    sender.send(opt)
                        .map_err(|_| (/* ignore the sender's fail */))
                        .map(move |_| map)
                },
            }
        }).map(|_| (/* drop the map */));
        (provider, Box::new(fut))
    }

    pub fn register<T: AsyncRead + Send + 'static>(&self, path: &str, reader: T) {
        let msg = InMsg::Register(path.to_owned(), Box::new(reader));
        self.worker.clone().send(msg).wait().expect("can't add file to a provider");
    }
}

impl Provider for OnceProvider {
    fn find_file<'a>(&self, path: &'a str) -> FindFuture {
        let (tx, rx) = oneshot::channel();
        let msg = InMsg::Find(path.to_owned(), tx);
        let fut = self.worker.clone().send(msg)
            .map_err(|_| oneshot::Canceled)
            .and_then(|_| rx)
            .map_err(|_| other("provider lost"));
        Box::new(fut)
    }
}


type FindFuture = Box<Future<Item=Option<SendFile>, Error=io::Error>>;

pub trait Provider: Send + Sync + 'static {
    fn find_file<'a>(&self, path: &'a str) -> FindFuture;
}

pub struct StaticService<T: Provider> {
    provider: Arc<T>,
    sender: mpsc::Sender<Job>,
}

impl<T: Provider> Service for StaticService<T> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        if req.method() == &Get {
            let path = req.path();
            let (tx, body) = Body::pair();
            let finder_fut = self.provider.find_file(path);
            let sender = self.sender.clone();
            let fut = finder_fut.map_err(hyper::Error::from).and_then(move |reader| {
                if let Some(reader) = reader {
                    let send_fut = sender.send((tx, reader));
                    let fut = send_fut
                        .map_err(|_| other("can't send task").into())
                        .map(|_| {
                            Response::new().with_body(body)
                        });
                    let fut: Self::Future = Box::new(fut);
                    fut
                } else {
                    let fut = futures::future::ok(
                        Response::new().with_status(StatusCode::NotFound)
                    );
                    let fut: Self::Future = Box::new(fut);
                    fut
                }
            });
            Box::new(fut)
        } else {
            let fut = futures::future::ok(
                Response::new().with_status(StatusCode::MethodNotAllowed)
            );
            Box::new(fut)
        }
    }
}

type Job = (mpsc::Sender<hyper::Result<Chunk>>, SendFile);

pub struct StaticNewService<T: Provider> {
    provider: Arc<T>,
    sender: mpsc::Sender<Job>,
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

pub fn serve<T: Provider>(addr: &SocketAddr, provider: T, runner: Box<Future<Item=(), Error=()>>) {
    let pool = CpuPool::new_num_cpus();
    let (tx, rx) = mpsc::channel(10);
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
        handle.spawn(runner);
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

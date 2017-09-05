#[macro_use] extern crate log;
extern crate futures;
extern crate tokio_io;
extern crate tokio_file_unix;
extern crate bytes;
extern crate hyper;

use std::io;
use std::thread;
use std::fs::File;
use std::net::SocketAddr;
use std::collections::HashMap;
use futures::{Future, IntoFuture, Stream, Sink};
use futures::sync::{oneshot, mpsc};
use tokio_io::codec::{FramedRead, Decoder};
use bytes::BytesMut;
use hyper::{Chunk, Body};
use hyper::server::{Http, Service, Request, Response};

struct StaticService {
    provider: mpsc::Sender<Msg>,
}

impl Service for StaticService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let path = req.path().to_owned();
        println!("PATH: {:?}", path);
        let provider = self.provider.clone();
        let (mime_tx, rx) = oneshot::channel();
        let (body_tx, body) = Body::pair();
        let send_file = Msg::SendFile {
            path: path,
            mime: mime_tx,
            body: body_tx,
        };
        let send_msg = provider.send(send_file)
            .map_err(|_| other("can't send task"));
        let get_mime = rx.map(move |mime| {
                Response::new().with_body(body)
            })
            .map_err(|_| other("can't get mime type"));
        let fut = send_msg.and_then(|_| get_mime).map_err(hyper::Error::from);
        Box::new(fut)
    }
}

fn other(desc: &str) -> io::Error {
    error!("{}", desc);
    io::Error::new(io::ErrorKind::Other, desc)
}

enum Msg {
    Register{
        path: String,
        file: File,
    },
    SendFile {
        path: String,
        mime: oneshot::Sender<()>,
        body: mpsc::Sender<hyper::Result<Chunk>>,
    },
}

pub struct Registrator {
    provider: mpsc::Sender<Msg>,
}

impl Registrator {
    pub fn register(&self, path: &str, file: File) {
        let path = path.to_owned();
        let msg = Msg::Register {
            path,
            file,
        };
        self.provider.clone().send(msg).wait().unwrap();
    }
}

pub fn serve(addr: SocketAddr) -> (thread::JoinHandle<hyper::Result<()>>, Registrator) {
    let (tx, rx) = mpsc::channel(10);

    let registrator = Registrator {
        provider: tx.clone(),
    };

    let generator = move || {
        Ok(StaticService {
            provider: tx.clone(),
        })
    };

    let handle = thread::spawn(move || {
        let server = Http::new().bind(&addr, generator).map(move |server| {

            let handle = server.handle();
            let map = HashMap::new();
            let registrator = rx.fold(map, move |mut map, msg| {
                match msg {
                    Msg::Register{path, file} => {
                        map.insert(path, file);
                    },
                    Msg::SendFile{path, mime, body} => {
                        let file = map.remove(&path);
                        if let Some(file) = file {
                            let send_mime = mime.send(())
                                .into_future()
                                .map_err(|_| other("can't read body"));
                            let file = tokio_file_unix::File::new_nb(file).expect("new unix file");
                            let file = file.into_io(&handle).expect("attach async reader");
                            let decoder = ChunkDecoder::new(1024);
                            let framed = FramedRead::new(file, decoder);
                            let job = framed.fold(body, |body, chunk| {
                                    body.send(Ok(chunk))
                                        .map_err(|_| other("can't send chunk to the channel"))
                                })
                                .map(|_| (/* drop the channel */));
                            let fut = send_mime
                                .and_then(|_| job)
                                .map_err(|_| ());
                            handle.spawn(fut);
                        } else {
                            let send_mime = mime.send(())
                                .into_future()
                                .map_err(|_| ());
                            handle.spawn(send_mime);
                        }
                    },
                }
                Ok(map)
            }).map(|_| (/* drop the map */));

            let handle = server.handle();
            handle.spawn(registrator);
            server

        }).unwrap();
        server.run()
    });

    (handle, registrator)
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

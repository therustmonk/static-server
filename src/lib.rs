#[macro_use] extern crate log;
extern crate futures;
extern crate hyper;

use std::io::{self, Read};
use std::thread;
use std::fs::File;
use std::net::SocketAddr;
use std::collections::HashMap;
use futures::{Future, IntoFuture, Stream, Sink};
use futures::sync::{oneshot, mpsc};
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

            let map = HashMap::new();
            let registrator = rx.fold(map, move |mut map, msg| {
                match msg {
                    Msg::Register{path, file} => {
                        map.insert(path, file);
                        mybox(Ok(map).into_future())
                    },
                    Msg::SendFile{path, mime, body} => {
                        let file = map.remove(&path);
                        if let Some(file) = file {
                            let send_mime = mime.send(())
                                .into_future();
                            thread::spawn(move || {
                                let mut file = file;
                                let mut buffer = [0; 1024];
                                loop {
                                    if let Ok(len) = file.read(&mut buffer) {
                                        if len == 0 {
                                            break;
                                        }
                                        let slice = &buffer[0..len];
                                        let chunk = Chunk::from(slice.to_vec());
                                        body.clone().send(Ok(chunk)).wait().unwrap();
                                    } else {
                                        body.clone().send(Err(other("stream corrupted").into())).wait().unwrap();
                                    }
                                }
                            });
                            mybox(send_mime.map(|_| map))
                        } else {
                            let send_mime = mime.send(())
                                .into_future();
                            mybox(send_mime.map(|_| map))
                        }
                    },
                }
            }).map(|_| (/* drop the map */));

            let handle = server.handle();
            handle.spawn(registrator);
            server

        }).unwrap();
        server.run()
    });

    (handle, registrator)
}

fn other(desc: &str) -> io::Error {
    error!("{}", desc);
    io::Error::new(io::ErrorKind::Other, desc)
}

fn mybox<F: Future + 'static>(f: F) -> Box<Future<Item=F::Item, Error=F::Error>> {
    Box::new(f)
}


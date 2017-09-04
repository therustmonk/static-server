extern crate futures;
extern crate futures_cpupool;
extern crate hyper;

use std::io::{self, Read};
use std::net::SocketAddr;
use std::sync::Arc;
use futures::future::FutureResult;
use futures_cpupool::CpuPool;
use hyper::{Body, Get, StatusCode};
use hyper::server::{Http, Service, NewService, Request, Response};

pub trait Provider: Send + Sync + 'static {
    fn find_file<'a>(&self, path: &'a str) -> Option<Box<Read>>;
}

pub struct StaticService<T: Provider> {
    provider: Arc<T>,
}

impl<T: Provider> Service for StaticService<T> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = FutureResult<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        if req.method() == &Get {
            let path = req.path();
            if let Some(reader) = self.provider.find_file(path) {
                let (tx, body) = Body::pair();
                futures::future::ok(
                    Response::new().with_body(body)
                )
            } else {
                futures::future::ok(
                    Response::new().with_status(StatusCode::NotFound)
                )
            }
        } else {
            futures::future::ok(
                Response::new().with_status(StatusCode::MethodNotAllowed)
            )
        }
    }
}


pub struct StaticNewService<T: Provider> {
    provider: Arc<T>,
}

impl<T: Provider> NewService for StaticNewService<T> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Instance = StaticService<T>;

    fn new_service(&self) -> Result<Self::Instance, io::Error> {
        Ok(StaticService {
            provider: self.provider.clone(),
        })
    }
}

pub fn serve<T: Provider>(addr: &SocketAddr, provider: T) -> hyper::Result<hyper::Server<StaticNewService<T>, Body>> {
    let pool = CpuPool::new_num_cpus();
    let new_service = StaticNewService {
        provider: Arc::new(provider),
    };
    Http::new().bind(&addr, new_service)
}

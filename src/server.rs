use std::sync::Arc;
use std::net::ToSocketAddrs;

use hyper::Server;
use hyper::server::{Listening, Request, Response};
use hyper::uri::RequestUri;
use hyper::header::ContentType;
use hyper::status::StatusCode;

use provider::{StaticProvider};

pub struct StaticWorker {
	listening: Listening,
}

impl Drop for StaticWorker {
	fn drop(&mut self) {
		let _ = self.listening.close();
	}
}

// TODO Add trait StaticMapCreater with fn(path) and update/read method
pub struct StaticServer {
	map: Arc<StaticProvider>,
}

impl StaticServer {

	pub fn new<T: StaticProvider>(map: T) -> Self {
		StaticServer { map: Arc::new(map) }
	}

	pub fn share<To: ToSocketAddrs>(&self, addr: To) -> Result<StaticWorker, ()> {
		let server = match Server::http(addr) {
			Ok(server) => server,
			Err(_) => return Err(()),
		};
		let map = self.map.clone();
		let handler = move |req: Request, mut res: Response| {
			match req.uri {
				RequestUri::AbsolutePath(apath) => {
					debug!("Request for static resource {}", apath);
					if let Some(spath) = apath.splitn(2, '?').next() {
						let mut spath = String::from(spath);
						if spath.ends_with("/") {
							spath.push_str("index.html")
						}
						match map.get_content(&spath) {
							Some(item) => {
								*res.status_mut() = StatusCode::Ok;
								res.headers_mut().set(ContentType(item.mime.clone()));
								&res.send(&item.payload).unwrap();
							},
							None => *res.status_mut() = StatusCode::NotFound,
						}
					} else {
						*res.status_mut() = StatusCode::BadRequest;
					}
				},
				_ => *res.status_mut() = StatusCode::BadRequest,
			}
			//res.end();
		};
		// IMPORTANT! Browser blocks when there isn't enough threads!!!
		// That's because workers can't accept incomming connections in keep-alive state.
		match server.handle_threads(handler, 20) {
			Ok(listening) => {
				let worker = StaticWorker{ listening: listening};
				Ok(worker)
			},
			Err(_) => return Err(()),
		}
	}
}

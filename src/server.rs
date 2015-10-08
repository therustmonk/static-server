use std::sync::Arc;
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use mime_guess::guess_mime_type;

use hyper::Server;
use hyper::server::{Listening, Request, Response};
use hyper::uri::RequestUri;
use hyper::header::ContentType;
use hyper::status::StatusCode;


pub type StaticMap = HashMap<String, Vec<u8>>;

pub trait StaicProvider: Sync + Send + 'static {
	fn get_content(&self, path: &str) -> Option<&Vec<u8>>;
}

impl StaicProvider for StaticMap {
	fn get_content(&self, path: &str) -> Option<&Vec<u8>> {
		self.get(path)
	}
}

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
	map: Arc<StaicProvider>,
}

impl StaticServer {

	pub fn new<T: StaicProvider>(map: T) -> Self {
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
	    		RequestUri::AbsolutePath(mut apath) => {
	    			debug!("Request for static resource {}", apath);
	    			if apath.ends_with("/") {
	    				apath.push_str("index.html")
	    			}
    				if let Some(spath) = apath.splitn(2, '?').next() {
		    			let mime = guess_mime_type(spath.as_ref());
						res.headers_mut().set(ContentType(mime));						    				
						match map.get_content(&spath) {
							Some(item) => {
								*res.status_mut() = StatusCode::Ok;
								&res.send(item).unwrap();
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

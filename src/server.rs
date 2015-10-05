use std::sync::Arc;
use std::collections::HashMap;
use std::net::ToSocketAddrs;

use mime_guess::guess_mime_type;

use hyper::Server;
use hyper::server::{Handler, Listening, Request, Response};
use hyper::uri::RequestUri;
use hyper::header::ContentType;
use hyper::status::StatusCode;


pub type StaticMap = HashMap<String, Vec<u8>>;

pub trait StaicProvider {
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
	map: Arc<StaticMap>,
}

impl StaticServer {

	pub fn new(map: StaticMap) -> Self {
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
	    		RequestUri::AbsolutePath(mut spath) => {
	    			if spath.ends_with("/") {
	    				spath.push_str("index.html")
	    			}
	    			let mime = guess_mime_type(spath.as_ref());
					res.headers_mut().set(ContentType(mime));						    				
					match map.get_content(&spath) {
						Some(item) => {
							*res.status_mut() = StatusCode::Ok;
							&res.send(item).unwrap();
						},
						None => *res.status_mut() = StatusCode::NotFound,
					}
				},
				_ => *res.status_mut() = StatusCode::BadRequest,
			};
		};
		match server.handle(handler) {
			Ok(listening) => {
				let worker = StaticWorker{ listening: listening};
				Ok(worker)
			},
			Err(_) => return Err(()),
		}
	}
}

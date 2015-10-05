use std::sync::Arc;
use std::net::ToSocketAddrs;

use mime_guess::guess_mime_type;

use hyper::Server;
use hyper::server::{Handler, Listening, Request, Response};
use hyper::uri::RequestUri;
use hyper::header::ContentType;
use hyper::status::StatusCode;

use provider::StaticMap;


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
	    	let (status, item) = match req.uri {
	    		RequestUri::AbsolutePath(mut spath) => {
	    			if spath.ends_with("/") {
	    				spath.push_str("index.html")
	    			}
	    			let mime = guess_mime_type(spath.as_ref());
					res.headers_mut().set(ContentType(mime));						    				
					match map.get(&spath) {
						Some(item) => (StatusCode::Ok, Some(item)),
						None => (StatusCode::NotFound, None),
					}
				},
				_ => (StatusCode::BadRequest, None),
			};
			*res.status_mut() = status;
			if let Some(content) = item {
				res.send(&content).unwrap();
			}
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

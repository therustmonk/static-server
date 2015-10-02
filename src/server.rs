use std::path::{Path};
use std::sync::Arc;
use std::net::ToSocketAddrs;

use hyper::{Server};
use hyper::server::{Handler, Listening, Request, Response};
use hyper::uri::RequestUri;
use hyper::header::{ContentType};
use hyper::status::StatusCode;


// Sync because used concurrent from handlers
pub trait StaticProvider: Sync + Send + 'static {
	// Count references, because provider can keep data in memory or return just allocated. Two ways.
	fn get_data(&self, path: &Path) -> Option<Arc<Vec<u8>>>;
}


pub struct StaticServer<SP: StaticProvider> {
	provider: Arc<SP>,
}

pub struct StaticThread {
	listening: Listening,
}

impl Drop for StaticThread {
	fn drop(&mut self) {
		match self.listening.close() {
			Ok(_) => (),
			Err(_) => (),
		}
	}
}

impl<SP: StaticProvider> StaticServer<SP> {

	pub fn new(provider: SP) -> Self {
		StaticServer { provider: Arc::new(provider) }
	}

	pub fn share<To: ToSocketAddrs>(&self, addr: To) -> Result<StaticThread, ()> {
		let server = match Server::http(addr) {
			Ok(server) => server,
			Err(_) => return Err(()),
		};
		let provider = self.provider.clone();
		let handler = move |req: Request, mut res: Response| {
	    	match req.uri {
	    		RequestUri::AbsolutePath(ref apath) => {
	    			let path = if apath == "/" {
	    				Path::new("index.html")
	    			} else {
	    				Path::new(apath)
	    			};
					match provider.get_data(&path) {
						Some(content) => {
							*res.status_mut() = StatusCode::Ok;

							let mime = match path.extension() {
								Some(ext) => match ext.to_str() {
									Some("html") => mime!(Text/Html),
									Some("js"  ) => mime!(Application/Javascript),
									Some("css" ) => mime!(Text/Css),
									_ => mime!(Text/Plain),
								},
								_ => mime!(Text/Plain),
							};
							res.headers_mut().set(ContentType(mime));
							res.send(&content).unwrap();
						},
						None => *res.status_mut() = StatusCode::NotFound,
					}    			
				},
				_ => *res.status_mut() = StatusCode::BadRequest,
			}
		};
		match server.handle(handler) {
			Ok(listening) => Ok(StaticThread{ listening: listening}),
			Err(_) => return Err(()),
		}
	}
}


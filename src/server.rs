use std::path::{Path};
use std::sync::Arc;
use std::net::ToSocketAddrs;

use mime_guess::guess_mime_type;

use hyper::{Server};
use hyper::server::{Handler, Listening, Request, Response};
use hyper::uri::RequestUri;
use hyper::header::{ContentType};
use hyper::status::StatusCode;


pub trait StaticProvider: Sync + Send + 'static {
	fn get_data(&self, path: &Path) -> Option<Arc<Vec<u8>>>;
}

pub struct StaticServer<SP: StaticProvider> {
	provider: Arc<SP>,
}

pub struct StaticWorker {
	listening: Listening,
}

impl Drop for StaticWorker {
	fn drop(&mut self) {
		let _ = self.listening.close();
	}
}

impl<SP: StaticProvider> StaticServer<SP> {

	pub fn new(provider: SP) -> Self {
		StaticServer { provider: Arc::new(provider) }
	}

	pub fn share<To: ToSocketAddrs>(&self, addr: To) -> Result<StaticWorker, ()> {
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
							let mime = guess_mime_type(path);
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
			Ok(listening) => {
				let worker = StaticWorker{ listening: listening};
				Ok(worker)
			},
			Err(_) => return Err(()),
		}
	}
}


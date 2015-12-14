extern crate static_server;

extern crate env_logger;

use std::time::Duration;
use std::thread::sleep;
use std::path::Path;

use static_server::provider;
use static_server::server;

use std::env;

fn main() {
	env_logger::init().unwrap();	

	let ref path = match env::args().nth(1) {
		Some(value) => value,
		None => ".".to_owned(),
	};

	let p = provider::provider_from_folder(Path::new(path));
	let s = server::StaticServer::new(p);
	let _ = s.share(("0.0.0.0", 8080));
	loop { sleep(Duration::from_millis(1000)) } // TODO Content reload on changes
}


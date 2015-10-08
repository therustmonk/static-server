extern crate static_server;

extern crate env_logger;

use std::thread::sleep_ms;
use std::path::Path;

use static_server::provider;
use static_server::server;

use std::env;

fn main() {
	env_logger::init().unwrap();	

    let ref path = match env::args().nth(1) {
    	Some(value) => value,
    	None => "examples/static".to_owned(),
    };

	let p = provider::provider_from_folder(Path::new(path));
	let s = server::StaticServer::new(p);
	let _ = s.share(("localhost", 8081));
	loop { sleep_ms(1000) }
}
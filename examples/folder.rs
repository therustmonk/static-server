extern crate static_server;

use std::thread::sleep_ms;
use std::path::Path;

use static_server::provider;
use static_server::server;

fn main() {
	let p = provider::provider_from_folder(Path::new("examples/static"));
	let s = server::StaticServer::new(p);
	let _ = s.share(("localhost", 8081));
	loop { sleep_ms(1000) }
}
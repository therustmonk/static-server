//! Plugable HTTP Static server
//!
//! Features:
//! * Server files from folder
//! * Server files from tar
//! * Can be used as standalone server (not yet implemented!)

#[macro_use]
extern crate mime;
extern crate hyper;

pub mod server;
pub mod provider;


#[test]
fn test_multiple_ports_with_one_provider() {
	use provider::FolderProvider;
	use server::StaticServer;
	let provider = FolderProvider::new(".");
	let server = StaticServer::new(provider);
	let mut shared = Vec::new();
	for delta in 0..20 {
		match server.share(("localhost", 12345 + delta)) {
			Ok(worker) => shared.push(worker),
			Err(_) => panic!("Worker creation fail."),
		}
	}
	std::thread::sleep_ms(1000); // waiting for WSAStartup done
}

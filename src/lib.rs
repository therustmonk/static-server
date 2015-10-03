//! Plugable HTTP Static server
//!
//! Features:
//!
//!  * Serve files from folder
//!  * Serve files from TAR-archive
//!  * Can be used as standalone server (not yet implemented!)
//!  * Possible to write own files provider

extern crate mime_guess;
extern crate hyper;
extern crate tar;

pub mod server;
pub mod provider;

pub use server::{StaticServer, StaticWorker, StaticProvider};
pub use provider::{FolderProvider, TarProvider};


#[test]
fn test_multiple_ports_with_one_provider() {
	let provider = FolderProvider::new(".");
	let server = StaticServer::new(provider);
	let mut shared = Vec::new();
	for delta in 0..20 {
		match server.share(("localhost", 12345 + delta)) {
			Ok(worker) => shared.push(worker),
			Err(_) => panic!("Worker creation fail."),
		}
	}
	std::thread::sleep_ms(1000);
}

#[test]
fn test_folder_provider() {
	let provider = FolderProvider::new(".");
	let server = StaticServer::new(provider);
	let _ = server.share("localhost:8081");
	// TODO Call
}

#[test]
fn test_tar_provider() {
	let provider = TarProvider::new("example.tar");
	let server = StaticServer::new(provider);
	let _ = server.share("localhost:8082");
	// TODO Check by client
}

#[test]
fn test_own_provider() {
	use std::path::{Path};
	use std::sync::Arc;
	struct AProvider;
	impl StaticProvider for AProvider {
		fn get_data(&self, _: &Path) -> Option<Arc<Vec<u8>>> {
			None
		}
	}
	let server = StaticServer::new(AProvider);
	let _ = server.share("localhost:8083");

}
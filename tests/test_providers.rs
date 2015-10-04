extern crate static_server;
extern crate hyper;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use static_server::provider;
use static_server::server;

use hyper::Client;
use hyper::status::StatusCode;
use hyper::header::{Connection, ContentType};

fn get_content(source: &str) -> (StatusCode, String, String) {
    let client = Client::new();

    let mut res = client.get(source)
        .header(Connection::close())
        .send().unwrap();

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

	let content_type = match res.headers.get::<ContentType>() {
		Some(mime) => format!("{}", mime),
		None => "".to_owned(),
	};
    (res.status, body, content_type)
}

fn check_equals(port: u16, resource: &str, source: &str, content_type: &str) {
	let resource = format!("http://localhost:{}{}", port, resource);
	let source = format!("examples/static{}", source);
	let resp = get_content(&resource);

	let mut f = File::open(source).unwrap();
	let mut s = String::new();
	f.read_to_string(&mut s).unwrap();
	assert_eq!(resp.0, StatusCode::Ok);
	assert_eq!(resp.1, s);
	assert_eq!(resp.2, content_type);
}

fn check_resources(port: u16) {
	check_equals(port, "/", "/index.html", "text/html");
	check_equals(port, "/style.css", "/style.css", "text/css");
	check_equals(port, "/js/app.js", "/js/app.js", "application/x-javascript");
}

#[test]
fn test_folder_provider() {
	let p = provider::provider_from_folder(Path::new("examples/static"));
	let s = server::StaticServer::new(p);
	let _ = s.share(("localhost", 8081));
	check_resources(8081);
}

#[test]
fn test_tar_provider() {
	let p = provider::provider_from_tar(Path::new("examples/static.tar"));
	let s = server::StaticServer::new(p);
	let _ = s.share(("localhost", 8082));
	check_resources(8082);
}

#[test]
fn test_multiple_ports_with_one_provider() {
	let p = provider::provider_from_folder(Path::new("examples/static"));
	let s = server::StaticServer::new(p);
	for delta in 0..20 {
		let _ = s.share(("localhost", 12345 + delta));
	}
	for delta in 0..20 {
		check_resources(12345 + delta);
	}
}

/*
#[test]
fn test_own_provider() {
}
*/
extern crate static_server;

use std::fs::File;
use static_server::{serve};

fn main() {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let (handle, registrator) = serve(addr);
    let file = File::open("Cargo.toml").unwrap();
    registrator.register("/example", file);
    println!("Started!");
    handle.join().unwrap().unwrap();
}

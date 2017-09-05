extern crate static_server;
extern crate tokio_file_unix;

use static_server::{serve, OnceProvider};

fn main() {
    let (provider, runner) = OnceProvider::new();
    let stdin = std::io::stdin();
    let file = tokio_file_unix::StdFile(stdin.lock());
    let file = tokio_file_unix::File::new_nb(file).unwrap();
    provider.register("/myfile", file);
    let addr = "127.0.0.1:8080".parse().unwrap();
    serve(&addr, provider, runner);
}

use std::process::Command;
use std::env;

fn main() {
	let root = env::var("CARGO_MANIFEST_DIR").unwrap();
	println!("{:?}", root);
    let mut command = Command::new("tar");
    command.current_dir(root)
    	.arg("cvf").arg("examples/static.tar")    	
    	.arg("-C").arg("examples/static")
    	.arg(".");
    println!("{:?}", command);
    let ret = command.status().unwrap();
    assert!(ret.success());
}


use std::io::{Read};
use std::fs::{self, File};
use std::sync::Arc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tar::Archive;

pub type StaticMap = HashMap<String, Arc<Vec<u8>>>;


pub fn provider_from_folder(path: &Path) -> StaticMap {
	let mut result = StaticMap::new();
	let path = PathBuf::from(path);
	read_dir_with_subdirs(path, "".to_owned(), &mut result);
	result
}

pub fn provider_from_tar(path: &Path) -> StaticMap {
	let mut result = StaticMap::new();
    let arch_file = File::open(path).unwrap();
    let arch = Archive::new(arch_file);	

	for file in arch.files().unwrap() {
		let mut file = file.unwrap();

		match file.header().size() {
			Ok(0) => (),
			Ok(size) => {
				
				let str_path = {
					let mut pb = PathBuf::new();
					let file_path = file.header().path().unwrap();
	    	    	pb.push(file_path);
	    	    	let mut s = String::new();
	    	    	s.push_str(pb.to_str().unwrap());
	    	    	s.remove(0); // Drop first `.` in `./filename.ext`
	    	    	s
	    	    };	    	    
	    	    let mut content = Vec::with_capacity(size as usize);
    	    	file.read_to_end(&mut content).unwrap();
    	    	result.insert(str_path, Arc::new(content));
			},
			_ => ()
		}
	}
	result
}

fn read_file_to_vec(path: &Path) -> Vec<u8> {
	match File::open(path) {
		Ok(mut file) => {
			let mut content = Vec::new();
			file.read_to_end(&mut content).unwrap();
			print!("{:?}", String::from_utf8(content.clone()).unwrap());
			content
		}
		Err(_) => {
			panic!("Can't read file.");
		},
	}	
}

fn read_dir_with_subdirs(path: PathBuf, prefix: String, map: &mut StaticMap) {
	for entry in fs::read_dir(&path).unwrap() {
		let entry = entry.unwrap();
		let file_name = entry.file_name();
		let new_path = path.join(&file_name);
		
		let mut new_prefix = prefix.clone();
		new_prefix.push('/');
		new_prefix.push_str(&file_name.to_str().unwrap());

		let meta = entry.metadata().unwrap();
		if meta.is_dir() {
			read_dir_with_subdirs(new_path, new_prefix, map);
		} else if meta.is_file() {
			let content = read_file_to_vec(&new_path);
			map.insert(new_prefix, Arc::new(content));
		}
	}
}


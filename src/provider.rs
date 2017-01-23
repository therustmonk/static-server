use std::io::{Read};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use mime::Mime;
use mime_guess::guess_mime_type;

use tar::Archive;

pub struct Content {
	pub mime: Mime,
	pub payload: Vec<u8>,
}

pub type StaticMap = HashMap<String, Content>;

pub trait StaticProvider: Sync + Send + 'static {
	fn get_content(&self, path: &str) -> Option<&Content>;
}

impl StaticProvider for StaticMap {
	fn get_content(&self, path: &str) -> Option<&Content> {
		self.get(path)
	}
}

pub struct TryRewrite {
	map: StaticMap,
	path: String,
}

impl TryRewrite {
	pub fn new(map: StaticMap, path: String) -> Self {
		TryRewrite { map: map, path: path }
	}
}

impl StaticProvider for TryRewrite {
	fn get_content(&self, path: &str) -> Option<&Content> {
		let result = self.map.get(path);
		if result.is_none() {
			self.map.get(&self.path)
		} else {
			result
		}
	}
}

pub fn provider_from_folder(path: &Path) -> StaticMap {
	let mut result = StaticMap::new();
	let path = PathBuf::from(path);
	read_dir_with_subdirs(path, "".to_owned(), &mut result);
	result
}

pub fn provider_from_tar(path: &Path) -> StaticMap {
	let mut result = StaticMap::new();
	let arch_file = File::open(path).unwrap();
	let mut arch = Archive::new(arch_file);	

	for file in arch.entries().unwrap() {
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
				let mut payload = Vec::with_capacity(size as usize);
				file.read_to_end(&mut payload).unwrap();
				let content = Content {
					mime: guess_mime_type(&str_path),
					payload: payload,
				};
				result.insert(str_path, content);
			},
			_ => ()
		}
	}
	result
}

fn read_file_to_vec(path: &Path) -> Content {
	match File::open(path) {
		Ok(mut file) => {
			let mut payload = Vec::new();
			file.read_to_end(&mut payload).unwrap();
			Content {
				mime: guess_mime_type(path),
				payload: payload,
			}
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
			map.insert(new_prefix, content);
		}
	}
}


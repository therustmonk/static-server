use tar::Archive;
use std::io::Read;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use server::StaticProvider;

pub struct TarProvider {
	map: HashMap<String, Arc<Vec<u8>>>,
}

impl TarProvider {
	pub fn new(path: &str) -> TarProvider {
	    let arch_file = File::open(path).unwrap();
	    let arch = Archive::new(arch_file);
		
		let mut static_files = HashMap::new();

		for file in arch.files().unwrap() {
			let mut file = file.unwrap();

			match file.header().size() {
				Ok(0) => (),
				Ok(size) => {
					
					let str_path = {
						let mut pb = PathBuf::new();
						let file_path = file.header().path().unwrap(); // .into_owned()
		    	    	pb.push(file_path);
		    	    	String::from(pb.to_str().unwrap())

		    	    };	    	    
		    	    let mut content = Vec::with_capacity(size as usize);
	    	    	file.read_to_end(&mut content).unwrap();
	    	    	static_files.insert(str_path, Arc::new(content));
				},
				_ => ()
			}
		}
		TarProvider { map: static_files }
	}
}

impl StaticProvider for TarProvider {
	fn get_data(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
		if let Some(p) = path.to_str() {
			match self.map.get(p) {
				Some(rc) => Some(rc.clone()), // Share data under refcount
				None => None,
			}
		} else {
			None
		}
	} 
}

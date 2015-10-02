use std::io::Read;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use server::{StaticProvider};


pub struct FolderProvider {
	folder: String,
}

impl FolderProvider {
	pub fn new(path: &str) -> FolderProvider {
		FolderProvider { folder: String::from(path) }
	}
}

impl StaticProvider for FolderProvider {
	fn get_data(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
		let mut current_path = PathBuf::from(&self.folder);
		current_path.push(path);
		match File::open(current_path) {
			Ok(mut file) => {
				let mut content = Vec::new();
				file.read_to_end(&mut content).unwrap();
				Some(Arc::new(content))
			}
			Err(_) => {
				None
			},
		}

	} 
}

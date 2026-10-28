use std::fs;
use std::io;
use std::path::PathBuf;

use crate::{OwnerData, Store, StoreError, from_json, to_json};

/// Store that keeps the JSON in a file. Saves replace it through a sibling
/// temporary file so a failed write leaves the saved records intact.
#[derive(Debug)]
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    /// Uses this path for records. The parent folder is created on save.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Store for FileStore {
    fn load(&self) -> Result<Option<OwnerData>, StoreError> {
        match fs::read_to_string(&self.path) {
            Ok(text) => from_json(&text).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(backend_error(error)),
        }
    }

    fn save(&self, data: &OwnerData) -> Result<(), StoreError> {
        let text = to_json(data)?;
        if let Some(parent) = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(backend_error)?;
        }
        let mut temporary = self.path.as_os_str().to_os_string();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        let saved = fs::write(&temporary, text).and_then(|()| fs::rename(&temporary, &self.path));
        if saved.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        saved.map_err(backend_error)
    }
}

fn backend_error(error: io::Error) -> StoreError {
    StoreError::Backend(error.to_string())
}

//! Sessions and sync checkpoints live outside the owner JSON schema.

use werdy_store::{StoreError, StoreError::Backend};

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum File {
    Session,
    Sync,
}

impl File {
    #[cfg(any(test, target_os = "android", target_arch = "wasm32"))]
    fn name(self) -> &'static str {
        match self {
            Self::Session => "session.json",
            Self::Sync => "sync.json",
        }
    }
}

pub trait Vault {
    fn read(&self, file: File) -> Result<Option<String>, StoreError>;
    fn write(&self, file: File, text: &str) -> Result<(), StoreError>;
    fn remove(&self, file: File) -> Result<(), StoreError>;
}

fn failure() -> StoreError {
    Backend("account storage failed".into())
}

pub struct UnavailableVault;
impl Vault for UnavailableVault {
    fn read(&self, _file: File) -> Result<Option<String>, StoreError> {
        Err(failure())
    }
    fn write(&self, _file: File, _text: &str) -> Result<(), StoreError> {
        Err(failure())
    }
    fn remove(&self, _file: File) -> Result<(), StoreError> {
        Err(failure())
    }
}

#[cfg(any(test, target_os = "android"))]
pub struct FileVault(pub std::path::PathBuf);

#[cfg(any(test, target_os = "android"))]
impl Vault for FileVault {
    fn read(&self, file: File) -> Result<Option<String>, StoreError> {
        match std::fs::read_to_string(self.0.join(file.name())) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(failure()),
        }
    }
    fn write(&self, file: File, text: &str) -> Result<(), StoreError> {
        std::fs::create_dir_all(&self.0).map_err(|_| failure())?;
        let path = self.0.join(file.name());
        let temporary = self.0.join(format!("{}.tmp", file.name()));
        std::fs::write(&temporary, text)
            .and_then(|()| std::fs::rename(&temporary, &path))
            .map_err(|_| failure())
    }
    fn remove(&self, file: File) -> Result<(), StoreError> {
        match std::fs::remove_file(self.0.join(file.name())) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(failure()),
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct WebVault;

#[cfg(target_arch = "wasm32")]
impl Vault for WebVault {
    fn read(&self, file: File) -> Result<Option<String>, StoreError> {
        browser_storage()?
            .get_item(&format!("werdy.{}", file.name().trim_end_matches(".json")))
            .map_err(|_| failure())
    }
    fn write(&self, file: File, text: &str) -> Result<(), StoreError> {
        browser_storage()?
            .set_item(
                &format!("werdy.{}", file.name().trim_end_matches(".json")),
                text,
            )
            .map_err(|_| failure())
    }
    fn remove(&self, file: File) -> Result<(), StoreError> {
        browser_storage()?
            .remove_item(&format!("werdy.{}", file.name().trim_end_matches(".json")))
            .map_err(|_| failure())
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Result<web_sys::Storage, StoreError> {
    web_sys::window()
        .ok_or_else(failure)?
        .local_storage()
        .map_err(|_| failure())?
        .ok_or_else(failure)
}

#[cfg(any(test, not(any(target_arch = "wasm32", target_os = "android"))))]
#[derive(Default)]
pub struct MemoryVault(std::cell::RefCell<std::collections::BTreeMap<File, String>>);

#[cfg(any(test, not(any(target_arch = "wasm32", target_os = "android"))))]
impl Vault for MemoryVault {
    fn read(&self, file: File) -> Result<Option<String>, StoreError> {
        Ok(self.0.borrow().get(&file).cloned())
    }
    fn write(&self, file: File, text: &str) -> Result<(), StoreError> {
        self.0.borrow_mut().insert(file, text.to_owned());
        Ok(())
    }
    fn remove(&self, file: File) -> Result<(), StoreError> {
        self.0.borrow_mut().remove(&file);
        Ok(())
    }
}

pub fn open() -> Result<std::rc::Rc<dyn Vault>, StoreError> {
    #[cfg(target_arch = "wasm32")]
    {
        Ok(std::rc::Rc::new(WebVault))
    }
    #[cfg(target_os = "android")]
    {
        let path = crate::platform::android_store_path().ok_or_else(failure)?;
        let folder = path.parent().ok_or_else(failure)?.to_path_buf();
        Ok(std::rc::Rc::new(FileVault(folder)))
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        Ok(std::rc::Rc::new(MemoryVault::default()))
    }
}

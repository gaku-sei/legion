use anyhow::Context;
use anyhow::Result;
use lgn_source_control::disk_blob_storage::DiskBlobStorage;
use lgn_source_control::BlobStorage;
use lgn_telemetry::prelude::*;
use std::path::PathBuf;

pub struct DiskCache {
    storage: DiskBlobStorage,
}

impl DiskCache {
    pub fn new() -> Result<Self> {
        let folder = std::env::var("LEGION_TELEMETRY_CACHE_DIRECTORY").with_context(|| {
            String::from("Error reading env variable LEGION_TELEMETRY_CACHE_DIRECTORY")
        })?;
        let directory = PathBuf::from(folder);
        if !directory.exists() {
            std::fs::create_dir_all(&directory)
                .with_context(|| format!("Error creating cache folder {}", directory.display()))?;
        }
        Ok(Self {
            storage: DiskBlobStorage {
                blob_directory: directory,
            },
        })
    }

    pub async fn get(&self, name: &str) -> Result<Option<Vec<u8>>> {
        if !self.storage.exists(name).await? {
            return Ok(None);
        }
        let buffer = self.storage.read_bin_blob(name).await?;
        Ok(Some(buffer))
    }

    pub async fn put(&self, name: &str, buffer: &[u8]) -> Result<()> {
        if !self.storage.exists(name).await? {
            self.storage.write_blob(name, buffer).await?;
        }
        Ok(())
    }

    pub async fn get_cached_object<T>(&self, name: &str) -> Option<T>
    where
        T: prost::Message + Default,
    {
        match self.get(name).await {
            Err(e) => {
                error!("Error reading {} from cache: {}", name, e);
                None
            }
            Ok(Some(buffer)) => match T::decode(&*buffer) {
                Ok(obj) => Some(obj),
                Err(e) => {
                    error!("Error reading {} from cache: {}", name, e);
                    None
                }
            },
            Ok(None) => None,
        }
    }

    pub async fn get_or_put<FOBJ, T>(&self, name: &str, f: FOBJ) -> Result<T>
    where
        FOBJ: std::future::Future<Output = Result<T>>,
        T: Default + prost::Message,
    {
        if let Some(obj) = self.get_cached_object::<T>(name).await {
            return Ok(obj);
        }
        let obj = f.await?;
        if let Err(e) = self.put(name, &obj.encode_to_vec()).await {
            error!("Error writing to call tree cache: {}", e);
        }
        Ok(obj)
    }
}

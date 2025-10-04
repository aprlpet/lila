use std::path::PathBuf;

use axum::body::Bytes;
use futures_util::Stream;
use sha2::{Digest, Sha256};
use tokio::{fs, io::AsyncWriteExt};

use crate::error::{AppError, Result};

#[derive(Clone)]
pub struct FileStorage {
    pub base_path: PathBuf,
}

impl FileStorage {
    pub async fn new(base_path: &str) -> Result<Self> {
        let path = PathBuf::from(base_path);
        fs::create_dir_all(&path).await?;
        Ok(Self { base_path: path })
    }

    fn get_object_path(&self, key: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let subdir = &hash[..2];
        self.base_path.join(subdir).join(&hash)
    }

    pub fn get_object_path_string(&self, key: &str) -> String {
        self.get_object_path(key).display().to_string()
    }

    #[allow(dead_code)]
    pub async fn write(&self, key: &str, data: Vec<u8>) -> Result<String> {
        let path = self.get_object_path(key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&path).await?;
        file.write_all(&data).await?;

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let etag = hex::encode(hasher.finalize());

        Ok(etag)
    }

    pub async fn write_stream<S, E>(
        &self,
        key: &str,
        mut stream: S,
        max_size: usize,
    ) -> Result<(String, i64)>
    where
        S: Stream<Item = std::result::Result<Bytes, E>> + Unpin,
        E: std::error::Error + Send + Sync + 'static,
    {
        use futures_util::StreamExt;

        let path = self.get_object_path(key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(&path).await?;
        let mut hasher = Sha256::new();
        let mut total_size: usize = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

            if total_size + chunk.len() > max_size {
                drop(file);
                let _ = fs::remove_file(&path).await;
                return Err(AppError::PayloadTooLarge(max_size));
            }

            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            total_size += chunk.len();
        }

        file.flush().await?;
        let etag = hex::encode(hasher.finalize());

        Ok((etag, total_size as i64))
    }

    pub async fn open(&self, key: &str) -> Result<fs::File> {
        let path = self.get_object_path(key);

        match fs::File::open(&path).await {
            Ok(file) => Ok(file),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(AppError::NotFound(key.to_string()))
            }
            Err(e) => Err(AppError::Io(e)),
        }
    }

    #[allow(dead_code)]
    pub async fn read(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.get_object_path(key);

        match fs::read(&path).await {
            Ok(data) => Ok(data),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(AppError::NotFound(key.to_string()))
            }
            Err(e) => Err(AppError::Io(e)),
        }
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        let path = self.get_object_path(key);

        match fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(AppError::NotFound(key.to_string()))
            }
            Err(e) => Err(AppError::Io(e)),
        }
    }
}

use std::sync::Arc;

use bytes::Bytes;
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, ObjectStoreExt};
use thiserror::Error;
use uuid::Uuid;

const UPLOADS_DIR: &str = "uploads";
const ORIGINAL_FILE: &str = "original_file";
const NORMALIZED_OUTPUT: &str = "normalized_output.json";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage configuration error: {0}")]
    Config(String),
    #[error("object store error: {0}")]
    ObjectStore(#[from] object_store::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct StorageService {
    store: Arc<dyn ObjectStore>,
}

pub fn original_file_key(job_id: Uuid) -> String {
    format!("{job_id}/{ORIGINAL_FILE}")
}

pub fn normalized_output_key(job_id: Uuid) -> String {
    format!("{job_id}/{NORMALIZED_OUTPUT}")
}

impl StorageService {
    pub async fn from_env() -> Result<Self, StorageError> {
        let backend = std::env::var("STORAGE_BACKEND").unwrap_or_else(|_| "local".to_owned());

        let store: Arc<dyn ObjectStore> = match backend.as_str() {
            "s3" => Arc::new(build_s3_store()?),
            "local" | "" => {
                tokio::fs::create_dir_all(UPLOADS_DIR).await?;
                Arc::new(
                    LocalFileSystem::new_with_prefix(UPLOADS_DIR)
                        .map_err(|error| StorageError::Config(error.to_string()))?,
                )
            }
            other => {
                return Err(StorageError::Config(format!(
                    "unsupported STORAGE_BACKEND: {other} (expected 'local' or 's3')"
                )));
            }
        };

        Ok(Self { store })
    }

    pub async fn put_object(&self, key: &str, data: Bytes) -> Result<(), StorageError> {
        let path = object_path(key)?;
        self.store.put(&path, data.into()).await?;
        Ok(())
    }

    pub async fn get_object(&self, key: &str) -> Result<Bytes, StorageError> {
        let path = object_path(key)?;
        let result = self.store.get(&path).await?;
        let bytes = result.bytes().await?;
        Ok(bytes)
    }

    pub async fn object_exists(&self, key: &str) -> Result<bool, StorageError> {
        let path = object_path(key)?;
        match self.store.head(&path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

fn build_s3_store() -> Result<object_store::aws::AmazonS3, StorageError> {
    let bucket = std::env::var("S3_BUCKET")
        .map_err(|_| StorageError::Config("S3_BUCKET is required when STORAGE_BACKEND=s3".into()))?;

    let region = std::env::var("S3_REGION")
        .or_else(|_| std::env::var("AWS_REGION"))
        .map_err(|_| {
            StorageError::Config(
                "S3_REGION or AWS_REGION is required when STORAGE_BACKEND=s3".into(),
            )
        })?;

    let mut builder = object_store::aws::AmazonS3Builder::new()
        .with_bucket_name(bucket)
        .with_region(region);

    if let (Ok(access_key_id), Ok(secret_access_key)) = (
        std::env::var("AWS_ACCESS_KEY_ID"),
        std::env::var("AWS_SECRET_ACCESS_KEY"),
    ) {
        builder = builder
            .with_access_key_id(access_key_id)
            .with_secret_access_key(secret_access_key);
    }

    if let Ok(endpoint) = std::env::var("S3_ENDPOINT") {
        builder = builder.with_endpoint(endpoint);
    }

    builder
        .build()
        .map_err(|error| StorageError::Config(error.to_string()))
}

fn object_path(key: &str) -> Result<ObjectPath, StorageError> {
  ObjectPath::parse(key).map_err(|error| StorageError::Config(error.to_string()))
}

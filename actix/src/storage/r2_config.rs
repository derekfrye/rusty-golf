use rusty_golf_core::storage::StorageError;

use crate::storage::r2_signing::SigV4Signer;
use crate::storage::r2::R2Storage;

#[derive(Clone, Debug)]
pub struct R2StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub service: String,
}

impl R2StorageConfig {
    /// Build config from `.env` and process environment variables.
    ///
    /// Required:
    /// - `R2_ENDPOINT`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`
    ///
    /// Optional:
    /// - `R2_REGION` (defaults to `auto`)
    /// - `R2_SERVICE` (defaults to `s3`)
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing.
    pub fn from_env() -> Result<Self, StorageError> {
        dotenvy::dotenv().ok();

        let endpoint =
            std::env::var("R2_ENDPOINT").map_err(|_| StorageError::new("missing R2_ENDPOINT"))?;
        let bucket =
            std::env::var("R2_BUCKET").map_err(|_| StorageError::new("missing R2_BUCKET"))?;
        let access_key_id = std::env::var("R2_ACCESS_KEY_ID")
            .or_else(|_| std::env::var("AWS_ACCESS_KEY_ID"))
            .map_err(|_| StorageError::new("missing R2_ACCESS_KEY_ID"))?;
        let secret_access_key = std::env::var("R2_SECRET_ACCESS_KEY")
            .or_else(|_| std::env::var("AWS_SECRET_ACCESS_KEY"))
            .map_err(|_| StorageError::new("missing R2_SECRET_ACCESS_KEY"))?;
        let region = std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string());
        let service = std::env::var("R2_SERVICE").unwrap_or_else(|_| "s3".to_string());

        Ok(Self {
            endpoint,
            bucket,
            region,
            access_key_id,
            secret_access_key,
            service,
        })
    }

    #[must_use]
    pub fn signer(&self) -> SigV4Signer {
        SigV4Signer::new(
            self.access_key_id.clone(),
            self.secret_access_key.clone(),
            self.region.clone(),
            self.service.clone(),
        )
    }
}

impl R2Storage {
    /// Build config from `.env` and process environment variables.
    ///
    /// # Errors
    /// Returns an error if required environment variables are missing.
    pub fn config_from_env() -> Result<R2StorageConfig, StorageError> {
        R2StorageConfig::from_env()
    }

    #[must_use]
    pub fn signer_from_config(config: &R2StorageConfig) -> SigV4Signer {
        config.signer()
    }
}

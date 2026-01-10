use aws_sign_v4::AwsSign;
use chrono::Utc;
use reqwest::Url;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rusty_golf_core::storage::StorageError;
use sha256::digest as sha256_digest;

#[derive(Debug)]
pub struct MissingSigner;

impl S3Signer for MissingSigner {
    fn sign(
        &self,
        _method: &str,
        _url: &str,
        _headers: HeaderMap,
        _body: Option<&[u8]>,
    ) -> Result<HeaderMap, StorageError> {
        Err(StorageError::new("S3 signer not configured for R2Storage"))
    }
}

#[derive(Clone)]
pub struct SigV4Signer {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    service: String,
}

impl SigV4Signer {
    #[must_use]
    pub fn new(
        access_key_id: String,
        secret_access_key: String,
        region: String,
        service: String,
    ) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
            service,
        }
    }
}

pub trait S3Signer: Send + Sync {
    /// Sign a request and return the headers to attach.
    ///
    /// # Errors
    /// Returns an error if the request cannot be signed.
    fn sign(
        &self,
        method: &str,
        url: &str,
        headers: HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<HeaderMap, StorageError>;
}

impl S3Signer for SigV4Signer {
    fn sign(
        &self,
        method: &str,
        url: &str,
        mut headers: HeaderMap,
        body: Option<&[u8]>,
    ) -> Result<HeaderMap, StorageError> {
        let body = body.unwrap_or(&[]);
        let url = Url::parse(url).map_err(|e| StorageError::new(format!("invalid url: {e}")))?;

        let host = url
            .host_str()
            .ok_or_else(|| StorageError::new("missing host in url"))?;
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let payload_hash = sha256_digest(body);

        headers.insert(
            HeaderName::from_static("host"),
            HeaderValue::from_str(host)
                .map_err(|e| StorageError::new(format!("invalid host header: {e}")))?,
        );
        headers.insert(
            HeaderName::from_static("x-amz-date"),
            HeaderValue::from_str(&amz_date)
                .map_err(|e| StorageError::new(format!("invalid x-amz-date: {e}")))?,
        );
        headers.insert(
            HeaderName::from_static("x-amz-content-sha256"),
            HeaderValue::from_str(&payload_hash)
                .map_err(|e| StorageError::new(format!("invalid x-amz-content-sha256: {e}")))?,
        );

        let signer = AwsSign::new(
            method,
            url.as_str(),
            &now,
            &headers,
            &self.region,
            &self.access_key_id,
            &self.secret_access_key,
            &self.service,
            body,
        );
        let auth_header = signer.sign();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&auth_header)
                .map_err(|e| StorageError::new(format!("invalid authorization: {e}")))?,
        );

        Ok(headers)
    }
}

use axum::http::request::Parts;
use axum_governor::extractor::{KeyExtractor, KeyOutcome};
use axum_governor::error::ExtractionError;

#[derive(Clone, Default)]
pub struct PathKeyExtractor;

impl KeyExtractor for PathKeyExtractor {
    type Key = String;

    fn extract(&self, parts: &Parts) -> Result<KeyOutcome<Self::Key>, ExtractionError> {
        // Read the path directly from the request parts
        let path = parts.uri.path().to_string();
        
        Ok(KeyOutcome {
            key: path,
            quota_override: None,
        })
    }
}

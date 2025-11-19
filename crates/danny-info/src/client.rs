//! HTTP client wrapper with rate limiting

use crate::error::Result;
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// Rate limiter for a specific registry
pub type RegistryRateLimiter = Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>;

/// HTTP client wrapper for making registry and API requests with rate limiting
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: reqwest::Client,
    rate_limiter: Option<RegistryRateLimiter>,
}

impl HttpClient {
    /// Create a new HTTP client with default configuration (no rate limiting)
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(format!("fob-info/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            rate_limiter: None,
        })
    }

    /// Create a new HTTP client with rate limiting
    ///
    /// # Arguments
    ///
    /// * `requests_per_second` - Maximum requests per second (e.g., 1 for crates.io)
    pub fn with_rate_limit(requests_per_second: u32) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent(format!("fob-info/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()?;

        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        Ok(Self {
            client,
            rate_limiter: Some(rate_limiter),
        })
    }

    /// Wait for rate limiter if enabled
    async fn wait_for_rate_limit(&self) {
        if let Some(limiter) = &self.rate_limiter {
            limiter.until_ready().await;
        }
    }

    /// Make a GET request and deserialize JSON response
    pub async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        self.wait_for_rate_limit().await;

        let response = self.client.get(url).send().await?;

        // Handle rate limiting (HTTP 429)
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(crate::error::Error::RateLimitExceeded(url.to_string()));
        }

        // Check if the request was successful
        if !response.status().is_success() {
            return Err(crate::error::Error::other(format!(
                "HTTP request failed with status {}: {}",
                response.status(),
                url
            )));
        }

        let json = response.json().await?;
        Ok(json)
    }

    /// Make a GET request with custom headers and deserialize JSON response
    pub async fn get_json_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        headers: reqwest::header::HeaderMap,
    ) -> Result<T> {
        self.wait_for_rate_limit().await;

        let response = self.client.get(url).headers(headers).send().await?;

        // Handle rate limiting (HTTP 429)
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(crate::error::Error::RateLimitExceeded(url.to_string()));
        }

        if !response.status().is_success() {
            return Err(crate::error::Error::other(format!(
                "HTTP request failed with status {}: {}",
                response.status(),
                url
            )));
        }

        let json = response.json().await?;
        Ok(json)
    }

    /// Make a GET request and return the response text
    pub async fn get_text(&self, url: &str) -> Result<String> {
        self.wait_for_rate_limit().await;

        let response = self.client.get(url).send().await?;

        // Handle rate limiting (HTTP 429)
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(crate::error::Error::RateLimitExceeded(url.to_string()));
        }

        if !response.status().is_success() {
            return Err(crate::error::Error::other(format!(
                "HTTP request failed with status {}: {}",
                response.status(),
                url
            )));
        }

        let text = response.text().await?;
        Ok(text)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

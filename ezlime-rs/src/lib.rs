//! # ezlime-rs
//!
//! A Rust client library for the [ezli.me](https://ezli.me) URL shortener API.
//!
//! This crate provides a simple interface to create shortened URLs using the ezli.me service.
//! To use this API, you'll need an API key from ezli.me.
//!
//! ## Getting an API Key
//!
//! If you're interested in using ezli.me for your own project via the API, please join the
//! [Discord server](https://discord.gg/MHzmYHnnsE) to request an API key.
//!
//! ## Example
//!
//! ```rust,no_run
//! # async fn example() -> Result<(), ezlime_rs::EzlimeApiError> {
//! use ezlime_rs::EzlimeApi;
//!
//! let api = EzlimeApi::new("your-api-key-here".to_string());
//! let original_url = "https://example.com/very/long/url";
//!
//! let shortened = api.create_short_url(original_url).await?;
//! println!("Shortened URL: {}", shortened);
//! # Ok(())
//! # }
//! ```
//!

use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Request payload for creating a shortened URL.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateLinkRequest {
    /// The original URL to be shortened.
    pub url: String,
}

/// Response from the ezli.me API after creating a shortened URL.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatedLinkResponse {
    /// The unique identifier for the shortened link.
    pub id: String,
    /// The complete shortened URL.
    pub shortened_url: String,
    /// The original URL that was shortened.
    pub original_url: String,
}

impl CreatedLinkResponse {
    /// Creates a new `CreatedLinkResponse` with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for the shortened link
    /// * `prefix` - The URL prefix (e.g., `https://ezli.me`)
    /// * `original_url` - The original URL that was shortened
    pub fn new(id: String, prefix: &str, original_url: String) -> Self {
        let shortened_url = format!("{}/{}", prefix, id);
        Self {
            id,
            shortened_url,
            original_url,
        }
    }
}

/// A client for interacting with the ezli.me API.
///
/// This struct provides a convenient interface for creating shortened URLs
/// using the ezli.me service. It manages the API endpoint, authentication,
/// and HTTP client internally.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example() -> Result<(), ezlime_rs::EzlimeApiError> {
/// use ezlime_rs::EzlimeApi;
///
/// let api = EzlimeApi::new("your-api-key".to_string());
/// let shortened = api.create_short_url("https://example.com").await?;
/// println!("Shortened URL: {}", shortened);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct EzlimeApi {
    url: String,
    key: String,
    client: reqwest::Client,
}

/// Errors that can occur when interacting with the ezli.me API.
#[derive(Debug, Error)]
pub enum EzlimeApiError {
    /// An error occurred during API configuration (e.g., invalid URL parsing).
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    /// An error occurred while sending the HTTP request or receiving the response.
    #[error("Request error: {0}")]
    RequestError(String),
    /// An error occurred while deserializing the API response.
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}

impl EzlimeApi {
    /// Creates a new `EzlimeApi` client with the default ezli.me endpoint.
    ///
    /// # Arguments
    ///
    /// * `key` - Your ezli.me API key for authentication
    ///
    /// # Example
    ///
    /// ```rust
    /// use ezlime_rs::EzlimeApi;
    ///
    /// let api = EzlimeApi::new("your-api-key".to_string());
    /// ```
    pub fn new(key: String) -> Self {
        Self {
            url: String::from("https://ezli.me"),
            key,
            client: reqwest::Client::new(),
        }
    }

    /// Sets a custom API endpoint URL.
    ///
    /// By default, the client uses `https://ezli.me`. Use this method to
    /// configure a different endpoint, such as for testing or self-hosted instances.
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL of the ezli.me API endpoint
    ///
    /// # Example
    ///
    /// ```rust
    /// use ezlime_rs::EzlimeApi;
    ///
    /// let api = EzlimeApi::new("your-api-key".to_string())
    ///     .with_url("https://custom.ezli.me");
    /// ```
    pub fn with_url(mut self, url: &str) -> Self {
        self.url = url.into();
        self
    }

    /// Creates a shortened URL using the ezli.me API.
    ///
    /// This method sends a request to the ezli.me API to create a shortened version
    /// of the provided URL.
    ///
    /// # Arguments
    ///
    /// * `original_link` - The URL to be shortened
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` containing the shortened URL on success, or an
    /// `EzlimeApiError` if the request fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The API endpoint URL is invalid (`ConfigurationError`)
    /// - The HTTP request fails (`RequestError`)
    /// - The response cannot be deserialized (`DeserializationError`)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), ezlime_rs::EzlimeApiError> {
    /// use ezlime_rs::EzlimeApi;
    ///
    /// let api = EzlimeApi::new("your-api-key".to_string());
    /// let shortened = api.create_short_url("https://example.com/long/url").await?;
    /// println!("Shortened URL: {}", shortened);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_short_url(&self, original_link: &str) -> Result<String, EzlimeApiError> {
        let url: Url = Url::parse(&format!("{}/link/create", self.url))
            .map_err(|e| EzlimeApiError::ConfigurationError(e.to_string()))?;

        let resp = self
            .client
            .post(url)
            .header("Authorization", self.key.clone())
            .json(&CreateLinkRequest {
                url: original_link.to_string(),
            })
            .send()
            .await
            .map_err(|e| EzlimeApiError::RequestError(e.to_string()))?
            .json::<CreatedLinkResponse>()
            .await
            .map_err(|e| EzlimeApiError::DeserializationError(e.to_string()))?;

        Ok(resp.shortened_url)
    }
}

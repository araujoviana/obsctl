use crate::error::log_api_response;
use colored::Colorize;
use anyhow::{Context, Result};
use log::error;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use hmac::{Hmac, Mac};
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Method, Response};
use sha1::Sha1;
use futures::future::join_all;

// HMAC-SHA1 type alias for OBS signing, Ouch.
type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
pub struct Credentials {
    pub ak: String,
    pub sk: String,
}

#[derive(Clone)]
enum ContentType {
    ApplicationXml,
}

impl ContentType {
    fn as_str(&self) -> &'static str {
        match self {
            ContentType::ApplicationXml => "application/xml",
        }
    }
}

/// Generates a string of parameters for the OBS url
macro_rules! query_params {
    ( $( $key:expr => $val:expr ),* $(,)? ) => {{
        let mut params = Vec::new();
        $(
            if let Some(v) = $val.as_deref() {
                params.push(format!("{}={}", $key, v));
            }
        )*
        if params.is_empty() {
            "".to_string()
        } else {
            format!("?{}", params.join("&"))
        }
    }};
}

/// Sends a request to create an OBS bucket.
pub async fn create_bucket(
    client: &Client,
    bucket_name: &str,
    region: &str,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!("http://{}.obs.{}.myhuaweicloud.com", bucket_name, region);
    let body = format!(
        "<CreateBucketConfiguration><Location>{region}</Location></CreateBucketConfiguration>"
    );
    let canonical_resource = format!("/{}/", bucket_name);

    let response = generate_request(
        client,
        Method::PUT,
        &url,
        credentials,
        body,
        Some(ContentType::ApplicationXml),
        &canonical_resource,
    )
    .await?;
    log_api_response(response).await
}

/// Sends a request to list all OBS buckets.
pub async fn list_buckets(client: &Client, region: &str, credentials: &Credentials) -> Result<()> {
    let url = format!("http://obs.{}.myhuaweicloud.com", region);
    let body = "".to_string();
    let canonical_resource = "/";

    let response = generate_request(
        client,
        Method::GET,
        &url,
        credentials,
        body,
        None,
        canonical_resource,
    )
    .await?;
    log_api_response(response).await
}

/// Sends a request to list all objects in a bucket.
pub async fn list_objects(
    client: &Client,
    bucket_name: &str,
    prefix: &Option<String>,
    marker: &Option<String>,
    region: &str,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!(
        "http://{}.obs.{}.myhuaweicloud.com/{}",
        bucket_name,
        region,
        query_params!(
            "prefix" => prefix,
            "marker" => marker,
        )
    );
    let body = "".to_string();
    let canonical_resource = format!("/{}/", bucket_name);

    let response = generate_request(
        client,
        Method::GET,
        &url,
        credentials,
        body,
        None,
        &canonical_resource,
    )
    .await?;
    log_api_response(response).await
}

/// Deletes a single bucket from OBS
pub async fn delete_bucket(
    client: &Client,
    bucket_name: &str,
    region: &str,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!(
        "http://{}.obs.{}.myhuaweicloud.com/",
    bucket_name,
    region);

    let body = "".to_string();
    let canonical_resource = format!("/{}/", bucket_name);

    let response = generate_request(
        client,
        Method::DELETE,
        &url,
        credentials,
        body,
        None,
        &canonical_resource,
    )
    .await?;
    log_api_response(response).await
}

/// Deletes multiple buckets asynchronously from OBS
pub async fn delete_multiple_buckets(
    client: &Client,
    buckets: Vec<String>,
    region: &str,
    credentials: &Credentials,
) -> Result<()> {
    let delete_futures = buckets.into_iter().map(|bucket_name| {
        let client = client.clone(); // reqwest's Client is an Arc internally which facilitates cloning
        let credentials = credentials.clone();
        let region = region.to_string();
        tokio::spawn(async move {
            if let Err(e) = delete_bucket(&client, &bucket_name, &region, &credentials).await {
                error!("{} '{}': {}", "Failed to delete bucket:".red().bold() ,bucket_name, e);
            }
        })
    }).collect::<Vec<_>>();


    join_all(delete_futures).await;

    Ok(())

}

/// Computes the HMAC-SHA1 signature for a canonical string.
fn generate_signature(credentials: &Credentials, canonical_string: &str) -> Result<String> {
    // Initialize HMAC with secret key (sk).
    let mut mac = HmacSha1::new_from_slice(credentials.sk.as_bytes())
        .context("Failed to initialize HMAC-SHA1 with SK bytes")?;

    // Process the canonical string.
    mac.update(canonical_string.as_bytes());

    // Base64-encode the resulting signature.
    Ok(general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

/// Constructs and sends a signed HTTP request to OBS.
async fn generate_request(
    client: &Client,
    method: Method,
    url: &str,
    credentials: &Credentials,
    body: String,
    content_type_header: Option<ContentType>,
    canonical_resource: &str,
) -> Result<Response> {
    // Get current time in required GMT format.
    let date_str = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let content_md5 = ""; // MD5 seems optional
    let content_type_canonical = content_type_header.as_ref().map_or("", |ct| ct.as_str());

    // Assemble the canonical string for signing.
    let canonical_string = format!(
        "{}\n{}\n{}\n{}\n{}",
        method.as_str(),
        content_md5,
        content_type_canonical,
        date_str,
        canonical_resource,
    );

    debug!("Canonical String for signing:\n{}", canonical_string);

    let signature = generate_signature(credentials, &canonical_string)
        .context("Failed to generate request signature")?;

    // Build HTTP headers.
    let mut headers = HeaderMap::new();
    headers.insert("Date", HeaderValue::from_str(&date_str)?);
    if let Some(ct) = content_type_header {
        headers.insert("Content-Type", HeaderValue::from_static(ct.as_str()));
    }
    // Add authorization header with AK and signature.
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("OBS {}:{}", credentials.ak, signature))?,
    );

    // Send the request.
    let res = client
        .request(method, url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .context("Failed to send request to OBS endpoint")?;

    Ok(res)
}

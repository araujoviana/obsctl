use crate::error::log_api_response;
use crate::error::log_api_response_legacy;
use crate::generate_xml_table_vector;
use crate::xml::BucketList;
use crate::xml::ObjectList;
use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use colored::Colorize;
use futures::future::join_all;
use hmac::{Hmac, Mac};
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use log::error;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Method, Response};
use sha1::Sha1;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

// TODO IMPORTANT! PARSE XML AND MAKE IT READABLE!!
// TODO IMPORTANT! UNIFY PLURAL COMMANDS WITH SINGULAR COMMANDS!!

// HMAC-SHA1 type alias for OBS signing, Ouch.
type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
pub struct Credentials {
    pub ak: String,
    pub sk: String,
}

// Workaround sending binary file data to the API
enum Body {
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Clone)]
enum ContentType {
    ApplicationXml,
    ApplicationOctetStream,
}

impl ContentType {
    fn as_str(&self) -> &'static str {
        match self {
            ContentType::ApplicationXml => "application/xml",
            ContentType::ApplicationOctetStream => "application/octet-stream",
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
    region: String,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!("http://{}.obs.{}.myhuaweicloud.com", bucket_name, region);
    let body = Body::Text(format!(
        "<CreateBucketConfiguration><Location>{region}</Location></CreateBucketConfiguration>"
    ));
    let canonical_resource = format!("/{}/", bucket_name);

    let response = generate_request(
        client,
        Method::PUT,
        &url,
        credentials,
        body,
        Some(ContentType::ApplicationXml),
        "",
        &canonical_resource,
    )
    .await?;
    log_api_response_legacy(response).await
}

/// Sends a request to list all OBS buckets.
pub async fn list_buckets(
    client: &Client,
    region: String,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!("http://obs.{}.myhuaweicloud.com", region);
    let body = Body::Text("".to_string());
    let canonical_resource = "/";

    let response = generate_request(
        client,
        Method::GET,
        &url,
        credentials,
        body,
        None,
        "",
        canonical_resource,
    )
    .await?;

    let status = response.status(); // extract before consuming
    let raw_xml = response
        .text()
        .await
        .context("Failed to read response body")?;

    let parsed = generate_xml_table_vector!(
        BucketList => "Bucket" in &raw_xml, {
            Name => name,
            CreationDate => creation_date,
            Location => location,
            BucketType => bucket_type
        }
    );

    log_api_response(status, parsed, raw_xml).await
}

// TODO add object filtering
/// Sends a request to list all objects in a bucket.
pub async fn list_objects(
    client: &Client,
    bucket_name: &str,
    prefix: &Option<String>,
    marker: &Option<String>,
    region: String,
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
    let body = Body::Text("".to_string());
    let canonical_resource = format!("/{}/", bucket_name);

    let response = generate_request(
        client,
        Method::GET,
        &url,
        credentials,
        body,
        None,
        "",
        &canonical_resource,
    )
    .await?;

    let status = response.status(); // extract before consuming
    let raw_xml = response
        .text()
        .await
        .context("Failed to read response body")?;

    let parsed = generate_xml_table_vector!(
        ObjectList => "Contents" in &raw_xml, {
            Key => key,
            LastModified => last_modified,
            Size => size,
            StorageClass => storage_class,
        }
    );

    log_api_response(status, parsed, raw_xml).await
}

/// Deletes a single bucket from OBS
pub async fn delete_bucket(
    client: &Client,
    bucket_name: &str,
    region: String,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!("http://{}.obs.{}.myhuaweicloud.com/", bucket_name, region);

    let body = Body::Text("".to_string());
    let canonical_resource = format!("/{}/", bucket_name);

    let response = generate_request(
        client,
        Method::DELETE,
        &url,
        credentials,
        body,
        None,
        "",
        &canonical_resource,
    )
    .await?;
    log_api_response_legacy(response).await
}

/// Deletes multiple buckets asynchronously from OBS
pub async fn delete_multiple_buckets(
    client: &Client,
    buckets: Vec<String>,
    region: String,
    credentials: &Credentials,
) -> Result<()> {
    let delete_futures = buckets
        .into_iter()
        .map(|bucket_name| {
            let client = client.clone(); // reqwest's Client is an Arc internally which facilitates cloning
            let credentials = credentials.clone();
            let region = region.to_string();
            tokio::spawn(async move {
                // Errors shouldn't stop other concurrent deletion tasks
                if let Err(e) = delete_bucket(&client, &bucket_name, region, &credentials).await {
                    error!(
                        "{} '{}': {}",
                        "Failed to delete bucket:".red().bold(),
                        bucket_name,
                        e
                    );
                }
            })
        })
        .collect::<Vec<_>>();

    // Wait until all API calls are made
    join_all(delete_futures).await;

    Ok(())
}

/// Upload an object to a bucket
pub async fn upload_object(
    client: &Client,
    bucket_name: &str,
    region: String,
    file_path: &str,
    object_path: &Option<String>,
    credentials: &Credentials,
) -> Result<()> {
    // Extract the filename from the path to use as the object key
    let object_name = match object_path {
        Some(custom_path) => custom_path.clone(),
        None => Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
            .ok_or_else(|| {
                anyhow!(
                    "Invalid or missing filename in path, and no object-path provided: {}",
                    file_path.blue()
                )
            })?,
    };

    let url = format!(
        "http://{}.obs.{}.myhuaweicloud.com/{}",
        bucket_name, region, object_name
    ); // Object name is not a query parameter

    // Read file content as raw bytes
    let body_bytes = fs::read(file_path)
        .with_context(|| format!("Failed to read file at path {}", file_path.blue()))?;

    let digest = md5::compute(&body_bytes);
    let content_md5 = general_purpose::STANDARD.encode(digest.as_ref());

    let canonical_resource = format!("/{}/{}", bucket_name, object_name);

    let response = generate_request(
        client,
        Method::PUT,
        &url,
        credentials,
        Body::Binary(body_bytes),
        Some(ContentType::ApplicationOctetStream),
        &content_md5,
        &canonical_resource,
    )
    .await?;

    log_api_response_legacy(response).await
}

/// Download an object from a bucket
pub async fn download_object(
    client: &Client,
    bucket_name: &str,
    region: String,
    object_path: &str,
    output_dir: &Option<String>,
    credentials: &Credentials,
) -> Result<()> {
    // Remove first '/' if present
    let object_path = if object_path.starts_with('/') {
        &object_path[1..]
    } else {
        object_path
    };

    let url = format!(
        "http://{}.obs.{}.myhuaweicloud.com/{}",
        bucket_name, region, object_path
    );
    let body = Body::Text("".to_string());
    let canonical_resource = format!("/{}/{}", bucket_name, object_path);

    let response = generate_request(
        client,
        Method::GET,
        &url,
        credentials,
        body,
        None,
        "",
        &canonical_resource,
    )
    .await?;

    if !response.status().is_success() {
        log_api_response_legacy(response).await?;
        return Err(anyhow!(
            "Failed to download object: Server returned non-success status."
        ));
    }

    // Read entire response body into a buffer
    let content = response
        .bytes()
        .await
        .context("Failed to read response body bytes")?;

    // Extracts object file name
    let filename = Path::new(object_path).file_name().ok_or_else(|| {
        anyhow!(
            "Could not determine filename from object path: {}",
            object_path.yellow()
        )
    })?;

    let output_directory = output_dir.as_deref().unwrap_or(".");
    let mut local_path = PathBuf::from(output_directory);

    // Create directories for output path
    fs::create_dir_all(&local_path).with_context(|| {
        format!(
            "Failed to write downloaded content to {}",
            local_path.display()
        )
    })?;
    local_path.push(filename);

    // Write object's contents to disk
    fs::write(&local_path, &content).with_context(|| {
        format!(
            "Failed to write downloaded content to {}",
            local_path.display()
        )
    })?;

    log::info!(
        "Successfully downloaded '{}' to '{}'",
        object_path.cyan(),
        local_path.display().to_string().green()
    );

    Ok(())
}

/// Upload multiple object to a bucket
pub async fn upload_multiple_objects(
    client: &Client,
    bucket_name: &str,
    region: String,
    file_paths: Vec<String>,
    credentials: &Credentials,
) -> Result<()> {
    // Follows same logic as other parallel functions
    let upload_futures = file_paths
        .into_iter()
        .map(|file_path| {
            let client = client.clone();
            let bucket_name = bucket_name.to_string();
            let region = region.to_string();
            let credentials = credentials.clone();

            tokio::spawn(async move {
                if let Err(e) = upload_object(
                    &client,
                    &bucket_name,
                    region,
                    &file_path,
                    &None,
                    &credentials,
                )
                .await
                {
                    error!("Failed to upload file '{}': {}", file_path.red(), e);
                } else {
                    log::info!("Successfully uploaded '{}'", file_path.green());
                }
            })
        })
        .collect::<Vec<_>>();

    // Wait until all API calls are made
    join_all(upload_futures).await;

    Ok(())
}

/// Delete an object from a bucket
pub async fn delete_object(
    client: &Client,
    bucket_name: &str,
    region: String,
    object_path: &str,
    credentials: &Credentials,
) -> Result<()> {
    let url = format!(
        "http://{}.obs.{}.myhuaweicloud.com/{}",
        bucket_name, region, object_path
    );
    let body = Body::Text("".to_string());
    let canonical_resource = format!("/{}/{}", bucket_name, object_path);

    let response = generate_request(
        client,
        Method::DELETE,
        &url,
        credentials,
        body,
        None,
        "",
        &canonical_resource,
    )
    .await?;

    log_api_response_legacy(response).await
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
    body: Body,
    content_type: Option<ContentType>,
    content_md5: &str,
    canonical_resource: &str,
) -> Result<Response> {
    // Spinner, for style
    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(Duration::from_millis(120));
    // Style from https://github.com/console-rs/indicatif/blob/main/examples/long-spinner.rs
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.red} {msg}")
            .unwrap()
            .tick_strings(&[
                "▹▹▹▹▹",
                "▸▹▹▹▹",
                "▹▸▹▹▹",
                "▹▹▸▹▹",
                "▹▹▹▸▹",
                "▹▹▹▹▸",
                "▪▪▪▪▪",
            ]),
    );
    spinner.set_message("Building canonical string...");

    // Get current time in required GMT format.
    let date_str = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let content_type_canonical = content_type.as_ref().map_or("", |ct| ct.as_str());

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

    spinner.set_message("Generating signature...");

    let signature = generate_signature(credentials, &canonical_string)
        .context("Failed to generate request signature")?;

    spinner.set_message("Building headers...");

    // Build HTTP headers.
    let mut headers = HeaderMap::new();
    headers.insert("Date", HeaderValue::from_str(&date_str)?);
    if let Some(ct) = content_type {
        headers.insert("Content-Type", HeaderValue::from_static(ct.as_str()));
    }
    // Add the Content-MD5 header if it's not empty.
    if !content_md5.is_empty() {
        headers.insert(
            "Content-MD5",
            HeaderValue::from_str(content_md5)
                .context("Couldn't convert content-md5 into string")?,
        );
    }

    // Add authorization header with AK and signature.
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("OBS {}:{}", credentials.ak, signature))?,
    );

    spinner.set_message("Calling OBS API...");

    // Send the request.
    let req = client.request(method, url).headers(headers);

    let req = match body {
        Body::Text(s) => req.body(s),
        Body::Binary(b) => req.body(b),
    };

    let res = req
        .send()
        .await
        .context("Failed to send request to OBS endpoint")?;

    spinner.finish_with_message("Done");

    Ok(res)
}

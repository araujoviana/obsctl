use crate::error::log_api_response;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use log::debug;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Method, Response};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

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

pub async fn create_bucket(
    client: &Client,
    bucket_name: &str,
    region: &str,
    credentials: &Credentials,
) -> Result<()> {
    let body = format!(
        "<CreateBucketConfiguration><Location>{region}</Location></CreateBucketConfiguration>"
    );
    let url = format!("http://{}.obs.{}.myhuaweicloud.com", bucket_name, region);
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

fn generate_signature(credentials: &Credentials, canonical_string: &str) -> Result<String> {
    let mut mac = HmacSha1::new_from_slice(credentials.sk.as_bytes())
        .context("Failed to initialize HMAC-SHA1 with SK bytes")?;
    mac.update(canonical_string.as_bytes());
    Ok(general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

async fn generate_request(
    client: &Client,
    method: Method,
    url: &str,
    credentials: &Credentials,
    body: String,
    content_type_header: Option<ContentType>,
    canonical_resource: &str,
) -> Result<Response> {
    let date_str = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let content_md5 = "";
    let content_type_canonical = content_type_header.as_ref().map_or("", |ct| ct.as_str());

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

    let mut headers = HeaderMap::new();
    headers.insert("Date", HeaderValue::from_str(&date_str)?);
    if let Some(ct) = content_type_header {
        headers.insert("Content-Type", HeaderValue::from_static(ct.as_str()));
    }
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("OBS {}:{}", credentials.ak, signature))?,
    );

    let res = client
        .request(method, url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .context("Failed to send request to OBS endpoint")?;

    Ok(res)
}

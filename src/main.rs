use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha1::Sha1; // OBS requires HMAC-SHA1 lol
use std::env;

type HmacSha1 = Hmac<Sha1>;

fn main() {
    let ak = env::var("HUAWEICLOUD_SDK_AK").expect("Oops");
    let sk = env::var("HUAWEICLOUD_SDK_SK").expect("Oops");

    let region = "la-south-2";
    let bucket_name = "testando2";
    let url = format!("http://{bucket_name}.obs.{region}.myhuaweicloud.com");
    let body = format!(
        "<CreateBucketConfiguration xmlns=\"http://obs.{region}.myhuaweicloud.com/doc/2015-06-30/\"><Location>{region}</Location></CreateBucketConfiguration>"
    );
    let date_str = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();
    let content_type = "application/xml";
    let canonical_string = format!("PUT\n\n{content_type}\n{date_str}\n/{bucket_name}/");

    let mut mac = HmacSha1::new_from_slice(sk.as_bytes()).unwrap();
    mac.update(canonical_string.as_bytes());
    let signature = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    let mut headers = HeaderMap::new();
    headers.insert("Date", HeaderValue::from_str(&date_str).unwrap());
    headers.insert("Content-Type", HeaderValue::from_static("application/xml"));
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("OBS {ak}:{signature}")).unwrap(),
    );

    let client = Client::new();
    let res = client.put(&url).headers(headers).body(body).send().unwrap();

    println!("{}", res.status());
    println!("{}", res.text().unwrap());
}

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use colored::*;
use csv::Reader;
use hmac::{Hmac, Mac};
use log::warn;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha1::Sha1; // OBS requires HMAC-SHA1 lol
use std::env;
use std::error::Error;
use std::fs::File;

type HmacSha1 = Hmac<Sha1>;

struct Credentials {
    ak: String,
    sk: String,
}

fn get_credentials() -> Result<Credentials> {
    // Could fail
    let ak = env::var("HUAWEICLOUD_SDK_AK");
    let sk = env::var("HUAWEICLOUD_SDK_SK");

    match (ak, sk) {
        (Ok(ak_val), Ok(sk_val)) => Ok(Credentials {
            ak: ak_val,
            sk: sk_val,
        }),
        _ => {
            warn!("HUAWEICLOUD_SDK_AK or HUAWEICLOUD_SDK_SK not found, checking 'credentials.csv' file");
            read_credentials_csv()
                .context("Couldn't extract AK/SK values from credentials.csv file")
        }
    }
}

fn read_credentials_csv() -> Result<Credentials> {
    // credentials.csv file follows a identical format, so we simplify
    // file parsing by reading the exact positions of the credentials

    let cred_file = File::open("credentials.csv").context("Cannot find credentials.csv")?;
    let mut rdr = csv::Reader::from_reader(cred_file);

    if let Some(result) = rdr.records().next() {
        let record = result.context("Can't find second line in csv")?;
        let ak = record.get(1).context("Missing AK in CSV")?.to_string();
        let sk = record.get(2).context("Missing SK in CSV")?.to_string();
        Ok(Credentials { ak, sk })
    } else {
        anyhow::bail!(format!(
        "\n{} Missing credentials.\nSet the environment variables {} and {} or provide a {} file in the current working directory where {} is executed.\n",
        "ERROR:".red().bold(),
        "HUAWEICLOUD_SDK_AK".yellow().bold(),
        "HUAWEICLOUD_SDK_SK".yellow().bold(),
        "credentials.csv".yellow().bold(),
        "obsctl".magenta().bold()
    ));
    }
}

fn main() {
    env_logger::init();

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

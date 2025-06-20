use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use colored::*;
use hmac::{Hmac, Mac};
use log::{debug, error, info, warn};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Response, Client};
use sha1::Sha1; // OBS requires HMAC-SHA1 lol
use std::env;
use std::fs::File;
use std::process::exit;

type HmacSha1 = Hmac<Sha1>;

const CONTENT_TYPE: &str = "application/xml";

macro_rules! obs_url {
    ($bucket_name:expr, $region:expr) => {
        format!("http://{}.obs.{}.myhuaweicloud.com", $bucket_name, $region)
    };
}

/// A command-line tool for file operations and management in Huawei Cloud OBS
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,

    /// OBS region (e.g. la-south-2). Required for all operations.
    #[arg(short, long)]
    region: String,

    /// Optional access key override. Use only if env var and credentials CSV are unavailable.
    #[arg(short, long)]
    ak: Option<String>,

    /// Optional secret key override. Use only if env var and credentials CSV are unavailable.
    #[arg(short, long)]
    sk: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a bucket
    Create(CreateArgs),
}

#[derive(Args)]
struct CreateArgs {
    #[arg(short, long)]
    bucket: String,
}

struct Credentials {
    ak: String,
    sk: String,
}

// Argument parsing

fn get_credentials() -> Result<Credentials> {
    info!("Reading AK/SK values from envvars");

    // may fail if env vars are unset
    let ak = env::var("HUAWEICLOUD_SDK_AK");
    let sk = env::var("HUAWEICLOUD_SDK_SK");

    match (ak, sk) {
        (Ok(ak_val), Ok(sk_val)) => Ok(Credentials {
            ak: ak_val,
            sk: sk_val,
        }),
        _ => {
            // fallback to CSV parsing if either env var is missing
            warn!("HUAWEICLOUD_SDK_AK or HUAWEICLOUD_SDK_SK not found, checking if 'credentials.csv' file is present in current directory");
            read_credentials_csv().with_context(|| {
                format!(
                    "\nMissing credentials.\nSet the environment variables {} and {}\nor provide a {} file in the current working directory where {} is executed.\n",
                    "HUAWEICLOUD_SDK_AK".yellow().bold(),
                    "HUAWEICLOUD_SDK_SK".yellow().bold(),
                    "credentials.csv".yellow().bold(),
                    "obsctl".magenta().bold()
                )
            })
        }
    }
}

fn read_credentials_csv() -> Result<Credentials> {
    info!("Reading AK/SK values from 'credentials.csv'");
    // credentials.csv is assumed to have a fixed structure,
    // so we read the second and third columns from the first data row directly

    let cred_file = File::open("credentials.csv").context("Cannot find credentials.csv")?;
    let mut rdr = csv::Reader::from_reader(cred_file);

    if let Some(result) = rdr.records().next() {
        let record = result.context("Can't find second line in csv")?;
        let ak = record.get(1).context("Missing AK in CSV")?.to_string(); // second column (index 1)
        let sk = record.get(2).context("Missing SK in CSV")?.to_string(); // third column (index 2)
        Ok(Credentials { ak, sk })
    } else {
        // file is empty or has only headers, no usable data
        anyhow::bail!("credentials.csv is present but contains no usable records");
    }
}

fn log_error_chain(err: anyhow::Error) {
    let mut msg = format!("{} {}", "ERROR:".red().bold(), err);
    for cause in err.chain().skip(1) {
        msg.push_str(&format!("\nCaused by: {}", cause));
    }
    error!("{}", msg);
}

async fn log_api_response(res: Response) -> Result<()> {
    let status = res.status();
    let body = res.text().await.context("Failed to read response body")?;
    let msg = format!("{} {}\n{}", "Result:".bright_green().bold(), status, body);
    info!("{}", msg);
    Ok(())
}

fn generate_signature(credentials: &Credentials, canonical_string: String) -> String {
    let mut mac = HmacSha1::new_from_slice(credentials.sk.as_bytes()).unwrap();
    mac.update(canonical_string.as_bytes());

    general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}


async fn generate_request(
    bucket: String,
    region: String,
    body: String,
    credentials: Credentials,
) -> Result<Response> {
    let date_str = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

    let canonical_string = format!("PUT\n\n{}\n{}\n/{}/", CONTENT_TYPE, date_str, bucket);
    let signature = generate_signature(&credentials, canonical_string);

    let mut headers = HeaderMap::new();
    headers.insert(
        "Date",
        HeaderValue::from_str(&date_str).context("Failed to insert date into headers")?,
    );
    headers.insert("Content-Type", HeaderValue::from_static(CONTENT_TYPE));
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("OBS {}:{}", credentials.ak, signature))
            .context("Failed to insert Authorization header")?,
    );

    let url = obs_url!(bucket, region);
    let client = Client::new();

    let res = client
        .put(&url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .context("Failed to send PUT request to OBS endpoint")?;

    Ok(res)
}

async fn create_bucket(
    bucket_name: String,
    region: String,
    credentials: Credentials,
) -> Result<Response> {
    let body = format!(
        "<CreateBucketConfiguration xmlns=\"http://obs.{region}.myhuaweicloud.com/doc/2015-06-30/\"><Location>{region}</Location></CreateBucketConfiguration>"
    );

    generate_request(bucket_name, region, body, credentials).await
}

async fn handle_create(bucket: String, region: String, credentials: Credentials) -> Result<()> {
    let res = create_bucket(bucket, region, credentials).await?;
    log_api_response(res).await?;
    Ok(())
}


#[tokio::main]
async fn main() {
    colog::init(); // Initialize logging backend
    debug!("Starting execution");

    let args = CliArgs::parse();

    debug!("CLI parsed succesfully");

    // Get AK/SK Credentials
    let credentials = match get_credentials() {
        Ok(creds) => creds,
        Err(e) => {
            log_error_chain(e);
            exit(1);
        }
    };

    match args.command {
        Commands::Create(create_args) => {
            debug!("Matched with create");
            if let Err(e) = handle_create(create_args.bucket, args.region, credentials).await {
                log_error_chain(e);
            }
        }
    }
}

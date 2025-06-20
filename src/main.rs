use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use colored::*;
use csv::Reader;
use hmac::{Hmac, Mac};
use log::{debug, error, info, warn};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Method, Response};
use sha1::Sha1;
use std::env;
use std::fs::File;
use std::process::exit;

// HMAC-SHA1 type alias for OBS authentication, legacy :O
type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
enum ContentType {
    ApplicationXml,
    // TextPlain, // Unused, kept for future use
    // ApplicationJson, // Unused
}

impl ContentType {
    fn as_str(&self) -> &'static str {
        match self {
            ContentType::ApplicationXml => "application/xml",
            // ContentType::TextPlain => "text/plain",
            // ContentType::ApplicationJson => "application/json",
        }
    }
}

// Argument parsing

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
    /// List buckets
    ListBuckets,
}

#[derive(Args)]
struct CreateArgs {
    /// Bucket name
    #[arg(short, long)]
    bucket: String,
}

struct Credentials {
    ak: String,
    sk: String,
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
    if status.is_success() {
        info!("{}", msg);
    } else {
        // Log successful but non-2xx responses as warnings or errors
        warn!("{}", msg);
    }
    Ok(())
}

/// Attempts to load AK/SK credentials, prioritizing CLI args, then env vars, then CSV file.
fn get_credentials(cli_ak: Option<String>, cli_sk: Option<String>) -> Result<Credentials> {
    // 1. Prioritize credentials from command-line arguments
    if let (Some(ak), Some(sk)) = (cli_ak, cli_sk) {
        info!("Reading AK/SK values from command-line arguments, consider using env vars instead");
        return Ok(Credentials { ak, sk });
    }

    // 2. Fallback to environment variables
    info!("Reading AK/SK values from envvars");
    let ak_env = env::var("HUAWEICLOUD_SDK_AK");
    let sk_env = env::var("HUAWEICLOUD_SDK_SK");

    if let (Ok(ak), Ok(sk)) = (ak_env, sk_env) {
        return Ok(Credentials { ak, sk });
    }

    // 3. Fallback to CSV file
    warn!("HUAWEICLOUD_SDK_AK or HUAWEICLOUD_SDK_SK not found, checking for 'credentials.csv' file");
    read_credentials_csv().with_context(|| {
        format!(
            "\nMissing credentials.\nProvide them via command-line flags (--ak, --sk),\nor set the environment variables {} and {},\nor provide a {} file in the current working directory.",
            "HUAWEICLOUD_SDK_AK".yellow().bold(),
            "HUAWEICLOUD_SDK_SK".yellow().bold(),
            "credentials.csv".yellow().bold(),
        )
    })
}

/// Reads AK/SK credentials from 'credentials.csv' assuming fixed CSV structure.
fn read_credentials_csv() -> Result<Credentials> {
    info!("Reading AK/SK values from 'credentials.csv'");
    let cred_file = File::open("credentials.csv").context("Cannot find credentials.csv")?;
    let mut rdr = Reader::from_reader(cred_file);

    if let Some(result) = rdr.records().next() {
        let record = result.context("Can't find a data record in the CSV file")?;
        let ak = record.get(1).ok_or_else(|| anyhow!("Missing AK in CSV (expected in second column)"))?.to_string();
        let sk = record.get(2).ok_or_else(|| anyhow!("Missing SK in CSV (expected in third column)"))?.to_string();
        Ok(Credentials { ak, sk })
    } else {
        bail!("credentials.csv is present but contains no usable records");
    }
}

/// Compute HMAC-SHA1 signature of the canonical string using the secret key (SK)
fn generate_signature(credentials: &Credentials, canonical_string: &str) -> Result<String> {
    let mut mac = HmacSha1::new_from_slice(credentials.sk.as_bytes())
        .context("Failed to initialize HMAC-SHA1 with SK bytes")?;
    mac.update(canonical_string.as_bytes());
    Ok(general_purpose::STANDARD.encode(mac.finalize().into_bytes()))
}

/// Builds and sends authenticated request to OBS with headers and body, returns response or error.
async fn generate_request(
    client: &Client, // Use a shared client
    method: Method,
    url: &str,
    credentials: &Credentials,
    body: String,
    content_type_header: Option<ContentType>,
    canonical_resource: &str,
) -> Result<Response> {
    let date_str = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

    let content_md5 = ""; // Per OBS docs, can be empty.
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

async fn create_bucket(
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

async fn list_buckets(client: &Client, region: &str, credentials: &Credentials) -> Result<()> {
    // The ListBuckets operation uses the regional service endpoint, not a bucket-specific one.
    let url = format!("http://obs.{}.myhuaweicloud.com", region);
    let body = "".to_string(); // GET requests have an empty body.
    let canonical_resource = "/";

    let response = generate_request(
        client,
        Method::GET,
        &url,
        credentials,
        body,
        None, // No Content-Type header needed for this GET request.
        canonical_resource,
    )
    .await?;
    log_api_response(response).await
}

#[tokio::main]
async fn main() -> Result<()> {
    colog::init(); // Initialize logging backend
    debug!("Starting execution");

    let args = CliArgs::parse();
    debug!("CLI parsed successfully");

    // Get AK/SK credentials
    let credentials = match get_credentials(args.ak, args.sk) {
        Ok(creds) => creds,
        Err(e) => {
            log_error_chain(e);
            exit(1);
        }
    };

    // Create a single reqwest client
    let client = Client::new();

    // The result of the command will determine the exit code
    let command_result = match args.command {
        Commands::Create(create_args) => {
            debug!("Executing 'create' command");
            create_bucket(&client, &create_args.bucket, &args.region, &credentials).await
        }
        Commands::ListBuckets => {
            debug!("Executing 'list-buckets' command");
            list_buckets(&client, &args.region, &credentials).await
        }
    };

    // If the command returned an error, log it and exit with a non-zero code
    if let Err(e) = command_result {
        log_error_chain(e);
        exit(1);
    }

    Ok(())
}

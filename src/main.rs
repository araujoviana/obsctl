mod auth; // Manages credential loading and validation.
mod cli; // Defines the command-line interface structure.
mod error; // Provides error handling and logging utilities.
mod obs; // Contains OBS API interaction logic.
mod xml; // Macros for XML-based structs and parsing

use std::process::exit;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use log::{debug, info, warn};
use reqwest::Client;
use strsim::levenshtein;

use crate::auth::get_credentials;
use crate::cli::{CliArgs, Commands};
use crate::error::log_error_chain;
use crate::obs::{
    // OBS operations
    create_bucket,
    delete_bucket,
    delete_multiple_buckets,
    delete_object,
    download_object,
    list_buckets,
    list_objects,
    upload_multiple_objects,
    upload_object,
};

// Maximum allowed edit distance for fuzzy region name matching
// 4 should be low enough to be unnoticeable
const LEVENSHTEIN_THRESHOLD: usize = 4;

// Huawei Cloud main regions and their project names
const HUAWEI_CLOUD_REGIONS: &[(&str, &str)] = &[
    ("santiago", "la-south-2"),
    ("johannesburg", "af-south-1"),
    ("bangkok", "ap-southeast-2"),
    ("hong-kong", "ap-southeast-1"),
    ("singapore", "ap-southeast-3"),
    ("beijing", "cn-north-4"),
    ("guiyang", "cn-southwest-2"),
    ("shanghai", "cn-east-3"),
    ("mexico-city", "la-north-1"),
    ("sao-paulo", "sa-brazil-1"),
    ("riyadh", "me-west-1"),
    ("istanbul", "tr-west-1"),
];

// Reduces boilerplate on command calls (a little bit)
macro_rules! exec_cmd {
    ($cmd_name:literal, $func:ident $(, $args:expr)*) => {{
        debug!("Executing '{}' command", $cmd_name);
        $func($($args),*).await
    }};
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize colog for logging without envvar setup
    colog::init();
    debug!("Starting execution");

    // Parse CLI arguments
    let args = CliArgs::parse();
    debug!("CLI parsed successfully");

    // Region is obligatory despite not being needed for some calls
    let project_name = fuzzy_match_region(&args.region.to_lowercase());

    // Load AK/SK keys
    let credentials = match get_credentials(args.ak, args.sk) {
        Ok(creds) => creds,
        Err(e) => {
            log_error_chain(e);
            exit(1);
        }
    };

    // Create a single, shared HTTP client
    let client = Client::new();

    // Execute command based on parsed subcommand.
    let command_result = match args.command {
        Commands::Create(sub_args) => exec_cmd!(
            "create",
            create_bucket,
            &client,
            &sub_args.bucket,
            project_name,
            &credentials
        ),
        Commands::ListBuckets => exec_cmd!(
            "list-buckets",
            list_buckets,
            &client,
            project_name,
            &credentials
        ),
        Commands::ListObjects(sub_args) => exec_cmd!(
            "list-objects",
            list_objects,
            &client,
            &sub_args.bucket,
            &sub_args.prefix,
            &sub_args.marker,
            project_name,
            &credentials
        ),
        Commands::DeleteBucket(sub_args) => exec_cmd!(
            "delete-bucket",
            delete_bucket,
            &client,
            &sub_args.bucket,
            project_name,
            &credentials
        ),
        Commands::DeleteBuckets(sub_args) => exec_cmd!(
            "delete-buckets",
            delete_multiple_buckets,
            &client,
            sub_args.buckets,
            project_name,
            &credentials
        ),
        Commands::UploadObject(sub_args) => exec_cmd!(
            "upload-object",
            upload_object,
            &client,
            &sub_args.bucket,
            project_name,
            &sub_args.file_path,
            &sub_args.object_path,
            &credentials
        ),
        Commands::UploadObjects(sub_args) => exec_cmd!(
            "upload-objects",
            upload_multiple_objects,
            &client,
            &sub_args.bucket,
            project_name,
            sub_args.files,
            &credentials
        ),
        Commands::DownloadObject(sub_args) => exec_cmd!(
            "download-object",
            download_object,
            &client,
            &sub_args.bucket,
            project_name,
            &sub_args.object_path,
            &sub_args.output_dir,
            &credentials
        ),
        Commands::DeleteObject(sub_args) => exec_cmd!(
            "delete-object",
            delete_object,
            &client,
            &sub_args.bucket,
            project_name,
            &sub_args.object_path,
            &credentials
        ),
    };

    // Logs bubbled up error context
    if let Err(e) = command_result {
        log_error_chain(e);
        exit(1);
    }

    Ok(())
}

/// Returns the Huawei Cloud project name matching input exactly or approximately.
fn fuzzy_match_region(input_region: &str) -> String {
    debug!("Pattern matching region");

    // Try to find an exact match of the project names
    if let Some((_, code)) = HUAWEI_CLOUD_REGIONS
        .iter()
        .find(|(_, code)| code == &input_region)
    {
        let name = code.to_string();
        info!("Exact matched project name {}", name.cyan());
        name
    } else {
        // Attempt fuzzy matching using levenshtein distance
        match HUAWEI_CLOUD_REGIONS
            .iter()
            .map(|(name, code)| (levenshtein(input_region, &name.to_lowercase()), code)) // Calculate distance between input and region names
            .filter(|(dist, _)| *dist <= LEVENSHTEIN_THRESHOLD)                           // Filter matches within allowed threshold
            .min_by_key(|(dist, _)| *dist)                                                // Pick the closest match
            .map(|(_, code)| code.to_string())                                            // Extract the equivalent project name
        {
            // Fuzzy match found
            Some(name) => {
                warn!(
                    "Fuzzy matched region {} into project name {}",
                    input_region.yellow(),
                    name.cyan()
                );
                name
            }
            // Unsatisfactory matches
            None => {
                let err = anyhow::anyhow!(
                    "Region '{}' not found or no close match within threshold",
                    input_region.red()
                );
                log_error_chain(err);
                std::process::exit(1);
            }
        }
    }
}

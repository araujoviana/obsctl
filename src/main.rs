mod auth; // Manages credential loading and validation.
mod cli; // Defines the command-line interface structure.
mod error; // Provides error handling and logging utilities.
mod obs; // Contains OBS API interaction logic.

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use log::debug;
use log::info;
use log::warn;
use obs::{
    delete_bucket, delete_multiple_buckets, delete_object, download_object, list_objects,
    upload_multiple_objects, upload_object,
};
use reqwest::Client;
use std::process::exit;
use strsim::levenshtein;

use crate::auth::get_credentials;
use crate::cli::{CliArgs, Commands};
use crate::error::log_error_chain;
use crate::obs::{create_bucket, list_buckets};

// Maximum allowed edit distance for fuzzy region name matching
// 4 should be low enough to be unnoticeable
const LEVENSHTEIN_THRESHOLD: usize = 4;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize colog for logging without envvar setup
    colog::init();
    debug!("Starting execution");

    // Parse CLI arguments
    let args = CliArgs::parse();
    debug!("CLI parsed successfully");

    // Region is obligatory despite not being needed for some calls
    let region_arg = args.region.to_lowercase();

    // Huawei Cloud main regions and their project names
    let regions = vec![
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

    // Resolve `project_name` by exact code match or fuzzy name match (max distance 3).
    // Panic if no suitable match found.
    debug!("Pattern matching region");
    let project_name = if let Some((_, code)) = regions.iter().find(|(_, code)| code == &region_arg)
    {
        let name = code.to_string();
        info!("{}", format!("Exact matched project name {}", name.cyan()));
        name
    } else {
        match regions
            .iter()
            .map(|(name, code)| (levenshtein(&region_arg, &name.to_lowercase()), code))
            .filter(|(dist, _)| *dist <= LEVENSHTEIN_THRESHOLD)
            .min_by_key(|(dist, _)| *dist)
            .map(|(_, code)| code.to_string())
        {
            Some(name) => {
                warn!(
                    "{}",
                    format!(
                        "Fuzzy matched region {} into project name {}",
                        region_arg.yellow(),
                        name.cyan()
                    )
                );
                name
            }
            None => {
                let err = anyhow::anyhow!(
                    "Region '{}' not found or no close match within threshold",
                    region_arg
                );
                log_error_chain(err);
                std::process::exit(1);
            }
        }
    };
    // Load AK/SK keys
    let credentials = match get_credentials(args.ak, args.sk) {
        Ok(creds) => creds,
        Err(e) => {
            log_error_chain(e);
            exit(1);
        }
    };

    // Create a single, shared HTTP client.
    let client = Client::new();

    // Execute command based on parsed subcommand.
    let command_result = match args.command {
        Commands::Create(sub_args) => {
            debug!("Executing 'create' command");
            create_bucket(&client, &sub_args.bucket, project_name, &credentials).await
        }
        Commands::ListBuckets => {
            debug!("Executing 'list-buckets' command");
            list_buckets(&client, project_name, &credentials).await
        }
        Commands::ListObjects(sub_args) => {
            debug!("Executing 'list-buckets' command");
            list_objects(
                &client,
                &sub_args.bucket,
                &sub_args.prefix,
                &sub_args.marker,
                project_name,
                &credentials,
            )
            .await
        }
        Commands::DeleteBucket(sub_args) => {
            debug!("Executing 'delete-bucket' command");
            delete_bucket(&client, &sub_args.bucket, project_name, &credentials).await
        }
        Commands::DeleteBuckets(sub_args) => {
            debug!("Executing 'delete-buckets' command");
            delete_multiple_buckets(&client, sub_args.buckets, project_name, &credentials).await
        }
        Commands::UploadObject(sub_args) => {
            debug!("Executing 'upload-object' command");
            upload_object(
                &client,
                &sub_args.bucket,
                project_name,
                &sub_args.file_path,
                &sub_args.object_path,
                &credentials,
            )
            .await
        }
        Commands::UploadObjects(sub_args) => {
            debug!("Executing 'upload-objects' command");
            upload_multiple_objects(
                &client,
                &sub_args.bucket,
                project_name,
                sub_args.files,
                &credentials,
            )
            .await
        }
        Commands::DownloadObject(sub_args) => {
            debug!("Executing 'download-object' command");
            download_object(
                &client,
                &sub_args.bucket,
                project_name,
                &sub_args.object_path,
                &sub_args.output_dir,
                &credentials,
            )
            .await
        }
        Commands::DeleteObject(sub_args) => {
            debug!("Executing 'download-object' command");
            delete_object(
                &client,
                &sub_args.bucket,
                project_name,
                &sub_args.object_path,
                &credentials,
            )
            .await
        }
    };

    if let Err(e) = command_result {
        log_error_chain(e);
        exit(1);
    }

    Ok(())
}

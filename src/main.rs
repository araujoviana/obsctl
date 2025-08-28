mod auth; // Manages credential loading and validation.
mod cli; // Defines the command-line interface structure.
mod config; // Configurations for the CLI
mod error; // Provides error handling and logging utilities.
mod obs; // Contains OBS API interaction logic.
mod xml; // Macros for XML-based structs and parsing

use std::process::exit;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use config::set_basic_configs;
use log::{debug, info, warn};
use reqwest::Client;
use strsim::levenshtein;

use crate::auth::get_credentials;
use crate::cli::{CliArgs, Commands};
use crate::error::log_error_chain;
use crate::obs::{
    // OBS operations
    create_bucket,
    delete_buckets,
    delete_object,
    download_object,
    list_buckets,
    list_objects,
    list_regions,
    upload_object,
    upload_objects,
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

#[tokio::main]
async fn main() -> Result<()> {
    colog::init();
    debug!("Starting execution");

    let args = CliArgs::parse();
    debug!("CLI parsed successfully");

    let command_result = match args.command {
        Commands::Setup => {
            debug!("Executing 'setup' command");
            set_basic_configs()?;
            Ok(())
        }
        _ => {
            let project_name = match args.region {
                Some(r) => fuzzy_match_region(&r.to_lowercase()),
                None => {
                    match std::env::var("HUAWEICLOUD_SDK_REGION") {
                        Ok(r) => {
                            info!("Using region from environment variable: {}", r.cyan());
                            fuzzy_match_region(&r.to_lowercase())
                        },
                        Err(_) => {
                            let err = anyhow::anyhow!(
                                "No region provided. Please provide a region using --region or set the HUAWEICLOUD_SDK_REGION environment variable. Run 'obsctl setup' to configure default credentials and region."
                            );
                            log_error_chain(err);
                            exit(1);
                        }
                    }
                }
            };

            let credentials = match get_credentials(args.ak, args.sk) {
                Ok(creds) => creds,
                Err(e) => {
                    log_error_chain(e);
                    exit(1);
                }
            };

            let client = Client::new();

            match args.command {
                Commands::Create(sub_args) => {
                    debug!("Executing 'create' command");
                    create_bucket(&client, &sub_args.bucket, project_name, &credentials).await
                }
                Commands::ListBuckets => {
                    debug!("Executing 'list-buckets' command");
                    list_buckets(&client, project_name, &credentials).await
                }
                Commands::ListObjects(sub_args) => {
                    debug!("Executing 'list-objects' command");
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
                    delete_buckets(&client, sub_args.buckets, project_name, &credentials).await
                }
                Commands::UploadObject(sub_args) => {
                    debug!("Executing 'upload-object' command");
                    if sub_args.file_paths.len() == 1 {
                        upload_object(
                            &client,
                            &sub_args.bucket,
                            project_name,
                            &sub_args.file_paths[0],
                            &sub_args.object_path,
                            &credentials,
                        )
                        .await
                    } else {
                        upload_objects(
                            &client,
                            &sub_args.bucket,
                            project_name,
                            sub_args.file_paths,
                            &credentials,
                        )
                        .await
                    }
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
                    debug!("Executing 'delete-object' command");
                    delete_object(
                        &client,
                        &sub_args.bucket,
                        project_name,
                        &sub_args.object_path,
                        &credentials,
                    )
                    .await
                }
                Commands::ListRegions => {
                    debug!("Executing 'list-regions' command");
                    list_regions(HUAWEI_CLOUD_REGIONS).await
                }
                _ => unreachable!(), // Should not happen as all commands are handled
            }
        }
    };

    if let Err(e) = command_result {
        log_error_chain(e);
        exit(1);
    }

    Ok(())
}

/// Returns the Huawei Cloud project name matching input exactly or approximately.
pub fn fuzzy_match_region(input_region: &str) -> String {
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

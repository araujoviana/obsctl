mod auth; // Manages credential loading and validation.
mod cli; // Defines the command-line interface structure.
mod error; // Provides error handling and logging utilities.
mod obs; // Contains OBS API interaction logic.

use anyhow::Result;
use clap::Parser;
use log::debug;
use obs::{
    delete_bucket, delete_multiple_buckets, delete_object, download_object, list_objects,
    upload_multiple_objects, upload_object,
};
use reqwest::Client;
use std::process::exit;

use crate::auth::get_credentials;
use crate::cli::{CliArgs, Commands};
use crate::error::log_error_chain;
use crate::obs::{create_bucket, list_buckets};

#[tokio::main]
async fn main() -> Result<()> {
    // Init colog for logging without env var setup
    colog::init();
    debug!("Starting execution");

    // Parse CLI arguments
    let args = CliArgs::parse();
    debug!("CLI parsed successfully");

    // Region is obligatory despite not being needed for some calls
    let region = args.region;

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
            create_bucket(&client, &sub_args.bucket, region, &credentials).await
        }
        Commands::ListBuckets => {
            debug!("Executing 'list-buckets' command");
            list_buckets(&client, region, &credentials).await
        }
        Commands::ListObjects(sub_args) => {
            debug!("Executing 'list-buckets' command");
            list_objects(
                &client,
                &sub_args.bucket,
                &sub_args.prefix,
                &sub_args.marker,
                region,
                &credentials,
            )
            .await
        }
        Commands::DeleteBucket(sub_args) => {
            debug!("Executing 'delete-bucket' command");
            delete_bucket(&client, &sub_args.bucket, region, &credentials).await
        }
        Commands::DeleteBuckets(sub_args) => {
            debug!("Executing 'delete-buckets' command");
            delete_multiple_buckets(&client, sub_args.buckets, region, &credentials).await
        }
        Commands::UploadObject(sub_args) => {
            debug!("Executing 'upload-object' command");
            upload_object(
                &client,
                &sub_args.bucket,
                region,
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
                region,
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
                region,
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
                region,
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

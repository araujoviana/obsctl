mod auth;
mod cli;
mod error;
mod obs;

use anyhow::Result;
use clap::Parser;
use cli::{CliArgs, Commands};
use log::debug;
use reqwest::Client;
use std::process::exit;

use crate::auth::get_credentials;
use crate::error::log_error_chain;
use crate::obs::{create_bucket, list_buckets};

#[tokio::main]
async fn main() -> Result<()> {
    colog::init();
    debug!("Starting execution");

    let args = CliArgs::parse();
    debug!("CLI parsed successfully");

    let credentials = match get_credentials(args.ak, args.sk) {
        Ok(creds) => creds,
        Err(e) => {
            log_error_chain(e);
            exit(1);
        }
    };

    let client = Client::new();

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

    if let Err(e) = command_result {
        log_error_chain(e);
        exit(1);
    }

    Ok(())
}

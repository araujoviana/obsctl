use crate::obs::Credentials;
use anyhow::{Context, Result, anyhow, bail};
use colored::*;
use csv::Reader;
use log::{debug, info, warn};
use std::env;
use std::fs::File;

/// Attempts to load AK/SK credentials, prioritizing CLI args, then env vars, then CSV file.
pub fn get_credentials(cli_ak: Option<String>, cli_sk: Option<String>) -> Result<Credentials> {
    debug!("Getting AK/SK credentials");

    // 1. Prioritize credentials from command-line arguments (which are not recommended)
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
    warn!(
        "HUAWEICLOUD_SDK_AK or HUAWEICLOUD_SDK_SK not found, checking for 'credentials.csv' file"
    );
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
    // Initialize CSV reader
    let mut rdr = Reader::from_reader(cred_file);

    if let Some(result) = rdr.records().next() {
        let record = result.context("Can't find a data record in the CSV file")?;
        let ak = record
            .get(1)
            .ok_or_else(|| anyhow!("Missing AK in CSV (expected in second column)"))?
            .to_string();
        let sk = record
            .get(2)
            .ok_or_else(|| anyhow!("Missing SK in CSV (expected in third column)"))?
            .to_string();
        Ok(Credentials { ak, sk })
    } else {
        bail!("credentials.csv is present but contains no usable records");
    }
}

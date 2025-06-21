use anyhow::{Context, Result};
use colored::*;
use log::{error, info, warn};
use reqwest::Response;

pub fn log_error_chain(err: anyhow::Error) {
    let mut msg = format!("{} {}", "ERROR:".red().bold(), err);
    for cause in err.chain().skip(1) {
        msg.push_str(&format!("\nCaused by: {}", cause));
    }
    error!("{}", msg);
}

pub async fn log_api_response(res: Response) -> Result<()> {
    let status = res.status();
    let body = res.text().await.context("Failed to read response body")?;
    let display_body = if body.trim().is_empty() {
        "No text in response body".bright_blue().to_string()
    } else {
        body
    };
    let msg = format!(
        "{} {}\n{}",
        "Result:".bright_green().bold(),
        status,
        display_body
    );
    if status.is_success() {
        info!("{}", msg);
    } else {
        warn!("{}", msg);
    }
    Ok(())
}

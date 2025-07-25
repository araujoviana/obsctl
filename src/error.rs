use anyhow::{Context, Result};
use colored::*;
use log::{error, info, warn};
use reqwest::{Response, StatusCode};
use tabled::{Table, settings::style::Style};

/// Logs an `anyhow::Error` and its causal chain.
pub fn log_error_chain(err: anyhow::Error) {
    let mut msg = format!("{} {}", "ERROR:".red().bold(), err);

    for cause in err.chain().skip(1) {
        msg.push_str(&format!("\nCaused by: {cause}"));
    }

    error!("{msg}");
}

/// Logs the status and body of an API response.
pub async fn log_api_response<T: tabled::Tabled>(
    status: StatusCode,
    parsed: Vec<T>,
    raw_body: String,
) -> Result<()> {
    let display_body = if raw_body.trim().is_empty() {
        // Empty XML
        "No text in response body".bright_blue().to_string()
    } else if parsed.is_empty() {
        // Empty table
        // FIXME errors aren't included so they could hide!
        "No entries in response table".bright_yellow().to_string()
    } else {
        // Table with entries
        let mut table = Table::new(parsed);
        table.with(Style::rounded());
        format!("{table}")
    };

    let msg = format!(
        "{} {}\n{}",
        "Result:".bright_green().bold(),
        status,
        display_body
    );

    // Log message based on status (despite API being inconsistent with error codes)
    if status.is_success() {
        info!("{msg}");
    } else {
        warn!("{msg}");
    }

    Ok(())
}

// TODO delete this
/// Logs the status and body of an API response.
pub async fn log_api_response_legacy(res: Response) -> Result<()> {
    let status = res.status();
    let body = res.text().await.context("Failed to read response body")?;

    let msg = if body.trim().is_empty() {
        format!(
            "{} {}\n{}",
            "Result:".bright_green().bold(),
            status,
            "No text in response body".bright_blue()
        )
    } else {
        format!("{} {}\n{}", "Result:".bright_green().bold(), status, body)
    };

    if status.is_success() {
        info!("{msg}");
    } else {
        warn!("{msg}");
    }

    Ok(())
}

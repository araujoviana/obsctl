use anyhow::Result;
use colored::*;
use log::{error, info, warn};
use reqwest::StatusCode;
use tabled::{settings::style::Style, Table, Tabled};

/// Logs an `anyhow::Error` and its causal chain.
pub fn log_error_chain(err: anyhow::Error) {
    let mut msg = format!("{} {}", "ERROR:".red().bold(), err);

    for cause in err.chain().skip(1) {
        msg.push_str(&format!("\nCaused by: {cause}"));
    }

    error!("{msg}");
}

/// Logs the status and body of an API response.
pub async fn log_api_response<T: Tabled>(
    status: StatusCode,
    parsed: Option<Vec<T>>,
    raw_body: &str,
) -> Result<()> {
    let display_body = if raw_body.trim().is_empty() {
        "No text in response body".bright_blue().to_string()
    } else if let Some(parsed_data) = parsed {
        if parsed_data.is_empty() {
            "No entries in response table".bright_yellow().to_string()
        } else {
            let mut table = Table::new(parsed_data);
            table.with(Style::rounded());
            format!("{table}")
        }
    } else {
        raw_body.to_string()
    };

    let msg = format!(
        "{} {}\n{}",
        "Result:".bright_green().bold(),
        status,
        display_body
    );

    if status.is_success() {
        info!("{msg}");
    } else {
        warn!("{msg}");
    }

    Ok(())
}
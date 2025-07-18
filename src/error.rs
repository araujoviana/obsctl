use anyhow::{Context, Result};
use colored::*;
use log::{error, info, warn};
use reqwest::Response;
use tabled::Table;
use xmltree::{Element, EmitterConfig};

use crate::xml::parse_bucket_list;

// TODO Print parsed and readable XML output

// HACK
fn pretty_print_xml(xml: String) -> Result<String> {
    let elem = Element::parse(xml.as_bytes()).context("Failed to parse XML")?;
    let mut out = Vec::new();
    elem.write_with_config(&mut out, EmitterConfig::new().perform_indent(true))
        .context("Failed to write XML")?; // REVIEW bad error message

    let output = Table::new(parse_bucket_list(&xml));
    println!("{output}");

    String::from_utf8(out).context("Failed to convert XML to String")
}

/// Logs an `anyhow::Error` and its causal chain.
pub fn log_error_chain(err: anyhow::Error) {
    let mut msg = format!("{} {}", "ERROR:".red().bold(), err);
    for cause in err.chain().skip(1) {
        msg.push_str(&format!("\nCaused by: {}", cause));
    }
    error!("{}", msg);
}

/// Logs the status and body of an API response.
pub async fn log_api_response(res: Response) -> Result<()> {
    let status = res.status();
    let body = res.text().await.context("Failed to read response body")?;
    let display_body = if body.trim().is_empty() {
        "No text in response body".bright_blue().to_string()
    } else {
        pretty_print_xml(body)?
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

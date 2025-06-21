use clap::{Args, Parser, Subcommand};

/// A command-line tool for file operations and management in Huawei Cloud OBS
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// OBS region (e.g. la-south-2). Required for all operations.
    #[arg(short, long)]
    pub region: String,

    /// Optional access key override. Use only if env var and credentials CSV are unavailable.
    #[arg(short, long)]
    pub ak: Option<String>,

    /// Optional secret key override. Use only if env var and credentials CSV are unavailable.
    #[arg(short, long)]
    pub sk: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a bucket
    Create(CreateArgs),
    /// List buckets
    ListBuckets,
}

#[derive(Args)]
pub struct CreateArgs {
    /// Bucket name
    #[arg(short, long)]
    pub bucket: String,
}

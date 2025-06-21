use clap::{Args, Parser, Subcommand};

/// A command-line tool for file operations and management in Huawei Cloud OBS
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// OBS region (e.g., la-south-2). Required for all operations, even region-independent ones.
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
    /// List all created buckets
    ListBuckets,
    /// List objects in a bucket
    ListObjects(ListObjectsArgs),
    /// Delete a bucket
    DeleteBucket(DeleteBucketArgs),
    /// Delete multiple buckets (experimental)
    DeleteBuckets(DeleteBucketsArgs),
}

#[derive(Args)]
pub struct CreateArgs {
    /// Bucket name
    #[arg(short, long)]
    pub bucket: String,
}

#[derive(Args)]
pub struct ListObjectsArgs {
    /// Bucket name
    #[arg(short, long)]
    pub bucket: String,
    /// Include only elements with the specified prefix
    #[arg(short, long)]
    pub prefix: Option<String>,
    /// List results after the object with the marker
    #[arg(short, long)]
    pub marker: Option<String>,
}

#[derive(Args)]
pub struct DeleteBucketArgs {
    /// Bucket name
    #[arg(short, long)]
    pub bucket: String,
}

#[derive(Args)]
pub struct DeleteBucketsArgs {
    /// Bucket name
    #[arg(short, long)]
    pub buckets: Vec<String>,
}

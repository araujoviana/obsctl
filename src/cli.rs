use clap::{Args, Parser, Subcommand};

// The message that appears when you use "--help"
const APP_HELP_TEMPLATE: &str = r"
         __              __  __
  ____  / /_  __________/ /_/ /
 / __ \/ __ \/ ___/ ___/ __/ /
/ /_/ / /_/ (__  ) /__/ /_/ /
\____/_.___/____/\___/\__/_/

{name} {version}
{author-with-newline}
{about-with-newline}
{usage-heading} {usage}

COMMANDS:
{subcommands}

OPTIONS:
{options}
";

/// A command-line tool for file operations and management in Huawei Cloud OBS
#[derive(Parser)]
#[command(version, about, long_about = None, help_template = APP_HELP_TEMPLATE, )]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// OBS region (e.g., la-south-2 or santiago). Required for all operations, even region-independent ones.
    #[arg(short, long)]
    pub region: String,

    /// Optional access key override. Use only if env var and credentials CSV are unavailable.
    #[arg(short, long, global = true)]
    pub ak: Option<String>,

    /// Optional secret key override. Use only if env var and credentials CSV are unavailable.
    #[arg(short, long, global = true)]
    pub sk: Option<String>,
}

// TODO list regions, just a big print, but useful

#[derive(Subcommand)]
pub enum Commands {
    // Multiple aliases are good but they pollute the menus too much
    /// Create a bucket
    #[command(visible_alias = "mkb")]
    Create(CreateArgs),

    /// List buckets in region
    #[command(visible_alias = "lsb")]
    ListBuckets,

    /// Delete a single bucket
    #[command(visible_alias = "rmb")]
    DeleteBucket(DeleteBucketArgs),

    /// List objects in a bucket
    #[command(visible_alias = "ls")]
    ListObjects(ListObjectsArgs),

    /// Upload an object to a bucket
    #[command(visible_alias = "put")]
    UploadObject(UploadObjectArgs),

    /// Download objects contents to disk
    #[command(visible_alias = "get")]
    DownloadObject(DownloadObjectArgs),

    /// Delete an object from a bucket
    #[command(visible_alias = "rm")]
    DeleteObject(DeleteObjectArgs),

    // TODO unify plural commands with normal commands
    /// (experimental) Delete multiple buckets
    #[command(visible_alias = "rmbs")]
    DeleteBuckets(DeleteBucketsArgs),

    /// (experimental) Upload multiple objects to a bucket
    #[command(visible_alias = "puts")]
    UploadObjects(UploadObjectsArgs),
}

// Arguments for commands

#[derive(Args)]
pub struct CreateArgs {
    /// The bucket to create
    pub bucket: String,
}

#[derive(Args)]
pub struct ListObjectsArgs {
    /// The bucket to list objects from
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
    /// The bucket to delete
    pub bucket: String,
}

#[derive(Args)]
pub struct DeleteBucketsArgs {
    /// List of (space separated) bucket names to delete
    #[arg(short, long, num_args = 1..)]
    pub buckets: Vec<String>,
}

#[derive(Args)]
pub struct UploadObjectArgs {
    /// The bucket to upload to
    pub bucket: String,
    /// File path
    #[arg(short, long)]
    pub file_path: String,
    /// Optional object path
    #[arg(short, long)]
    pub object_path: Option<String>,
}

#[derive(Args)]
pub struct UploadObjectsArgs {
    /// The bucket to upload to
    #[arg(short, long)]
    pub bucket: String,
    /// One or more local file paths to upload. The object key will be the filename.
    #[arg(short, long, num_args(1..))]
    pub files: Vec<String>,
}

#[derive(Args)]
pub struct DownloadObjectArgs {
    /// The bucket to download from
    pub bucket: String,
    /// Object path in bucket
    #[arg(short, long)]
    pub object_path: String,
    /// Output directory, NOT the filename
    #[arg(short = 'd', long)]
    pub output_dir: Option<String>,
}

#[derive(Args)]
pub struct DeleteObjectArgs {
    /// The bucket where the object is
    pub bucket: String,
    /// Object path in bucket
    #[arg(short, long)]
    pub object_path: String,
}

# obsctl

A blazingly fast command-line interface (CLI) for managing Huawei Cloud Object Storage Service (OBS) 🚀

Its focus is on making common operations as fast and simple as possible. It is not intended to be a feature-rich replacement for the official `obsutil`. For complex workflows, batch operations, or advanced configurations, using the official tool is recommended.

> **Disclaimer:** This is an unofficial, community-driven tool and is not affiliated with, endorsed by, or supported by Huawei. For official resources, please refer to the [Huawei Cloud website](https://www.huaweicloud.com/).

## Features

-   **Bucket Management**: Create, list, and delete buckets.
-   **Object Management**: Upload, download, delete, and list objects.
-   **(some) Parallel Operations**: Upload or delete multiple objects/buckets concurrently.
-   **Flexible Authentication**: Load credentials from command-line flags, environment variables, or a `credentials.csv` file.
-   **Note on API Output (Work in Progress)**: The tool currently prints the raw XML response from the OBS API. While functional, this output can be verbose and difficult to read. Future versions will include automatic parsing and pretty-printing to display this information in a clear, human-readable format.

## Installation

### Prerequisites

Ensure you have the Rust toolchain installed. If not, get it from [rustup.rs](https://rustup.rs/).

### From Crates.io (Coming Soon)

This tool will be available on `crates.io` soon. Once published, you will be able to install it with:
```bash
cargo install obsctl
```

### From Source

```bash
git clone https://github.com/your-username/obsctl.git
cd obsctl
cargo install --path .
```

## Quick Setup

### Authentication

`obsctl` loads your Huawei Cloud Access Key (AK) and Secret Key (SK) in the following order of priority:

1.  **Command-line flags** (least secure, use for quick tests only):
    ```bash
    obsctl -r <region> --ak <YOUR_AK> --sk <YOUR_SK> lsb
    ```

2.  **Environment variables** (recommended for general use):
    ```bash
    export HUAWEICLOUD_SDK_AK="YOUR_ACCESS_KEY"
    export HUAWEICLOUD_SDK_SK="YOUR_SECRET_KEY"
    ```

3.  **`credentials.csv` file**:
    You can get this file from your cloud console or create it yourself. `obsctl` expects the Access Key in the second column and the Secret Key in the third. For example:
    ```csv
    User Name,Access Key Id,Secret Access Key
    your-user,YOUR_ACCESS_KEY,YOUR_SECRET_KEY
    ```

## Usage

The basic command structure is `obsctl -r <region> <COMMAND> [ARGS]`.

### Examples

**List all buckets:**
```bash
obsctl -r us-east-3 lsb
```

**Create a new bucket:**
```bash
obsctl -r us-east-3 mkb -b my-new-bucket
```

**Upload a local file:** (Object name defaults to filename)
```bash
obsctl -r us-east-3 put -b my-new-bucket -f ./local/image.png
```

**Upload a file with a custom object path:**
```bash
obsctl -r us-east-3 put -b my-new-bucket -f ./image.png -o "archive/2025/image.png"
```

**Upload all `.jpg` files in the current directory in parallel:**
```bash
# Note: Your shell expands *.jpg into a list of files. This behavior may
# differ on Windows (cmd.exe).
obsctl -r us-east-3 puts -b my-new-bucket -f *.jpg
```

**List objects in a bucket:**
```bash
obsctl -r us-east-3 ls -b my-new-bucket
```

**Download an object:**```bash
obsctl -r us-east-3 get -b my-new-bucket -o "archive/2025/image.png" -d ~/Downloads
```

**Delete an object:**
```bash
obsctl -r us-east-3 rm -b my-new-bucket -o "archive/2025/image.png"
```

## Commands

| Command | Alias | Description                               |
| :------ | :---- | :---------------------------------------- |
| `create`  | `mkb` | Create a new bucket.                      |
| `list-buckets`|`lsb` | List all buckets.                         |
| `delete-bucket`|`rmb`| Delete a single bucket.                   |
| `list-objects`|`ls` | List objects within a bucket.             |
| `upload-object`|`put`| Upload a local file to a bucket.        |
| `download-object`|`get`| Download an object to disk.               |
| `delete-object`|`rm`| Delete an object from a bucket.           |
| `delete-buckets`|`rmbs`| (Experimental) Delete multiple buckets.   |
| `upload-objects`|`puts`| (Experimental) Upload multiple objects.   |


## License

This project is licensed under the [MIT license](http://opensource.org/licenses/MIT).

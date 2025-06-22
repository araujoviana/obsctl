# obsctl

A blazingly fast command-line interface (CLI) for managing Huawei Cloud Object Storage Service (OBS) 🚀

Its focus is on making common operations as fast and simple as possible. It is not intended to be a feature-rich replacement for the official `obsutil`. For complex workflows, batch operations, or advanced configurations, using the official tool is recommended.

> **Disclaimer:** This is an unofficial, community-driven tool and is not affiliated with, endorsed by, or supported by Huawei. For official resources, please refer to the [Huawei Cloud website](https://www.huaweicloud.com/).

## Features

-   **Bucket Management**: Create, list, and delete buckets.
-   **Object Management**: Upload, download, and list objects.
-   **Parallel Operations**: Upload or delete multiple objects/buckets concurrently.
-   **Flexible Authentication**: Load credentials from command-line flags, environment variables, or a `credentials.csv` file.

## Installation

### Prerequisites

Ensure you have the Rust toolchain installed. If not, get it from [rustup.rs](https://rustup.rs/).

> I will upload it to crates.io soon, sorry 🙃

### From Source

```bash
git clone https://github.com/your-username/obsctl.git
cd obsctl
cargo install --path .
```

## Setup

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
    You can get this file from your cloud console, *OR* you can create a file named `credentials.csv` in the directory where you run the command. The format should be:
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
# Your shell (untested on Windows) expands *.jpg into a list of files
obsctl -r us-east-3 puts -b my-new-bucket -f *.jpg
```

**List objects in a bucket:**
```bash
obsctl -r us-east-3 lso -b my-new-bucket
```

**Download an object:**
```bash
obsctl -r us-east-3 get -b my-new-bucket -o "archive/2025/image.png" -d ~/Downloads
```

## Commands (for now)

> `delete-object` will be added soon

| Command | Alias | Description                               |
| :------ | :---- | :---------------------------------------- |
| `create`  | `mkb` | Create a new bucket.                      |
| `list-buckets`|`lsb` | List all buckets.                         |
| `list-objects`|`lso` | List objects within a bucket.             |
| `delete-bucket`|`rmb`| Delete a single bucket.                   |
| `upload-object`|`put`| Upload a local file to a bucket.        |
| `download-object`|`get`| Download an object to disk.               |
| `delete-buckets`|`rmbs`| (Experimental) Delete multiple buckets.   |
| `upload-objects`|`puts`| (Experimental) Upload multiple objects.   |

## Disclaimer, again

This project is an independent tool developed by the community. It is not officially affiliated with, maintained, authorized, or endorsed by Huawei Technologies Co., Ltd. or any of its affiliates. All Huawei Cloud product names, logos, and brands are property of their respective owners.

This software is provided "as-is" and without warranty. Use at your own risk.

## License

[MIT license](http://opensource.org/licenses/MIT)

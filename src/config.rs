use anyhow::{Context, Result, anyhow};
use dialoguer::Input;
use log::info;
use std::fs::OpenOptions;
use std::io::Write;

use crate::fuzzy_match_region;

pub fn set_basic_configs() -> Result<()> {
    info!(
        "This command will guide you through setting up your Huawei Cloud OBS credentials (Access Key and Secret Key) and a default region. These will be saved as environment variables in your shell's profile (.bashrc, .zshrc, etc. on Linux/macOS, or PowerShell profile on Windows) so you don't have to specify them with every command. You can always override these settings using the --ak, --sk, and --region flags."
    );

    let ak: String = Input::new()
        .with_prompt("Paste your AK")
        .interact_text()
        .context("Invalid input")?;

    let sk: String = Input::new()
        .with_prompt("Paste your SK")
        .interact_text()
        .context("Invalid input")?;

    let mut region: String = Input::new()
        .with_prompt("Write your default region")
        .interact_text()
        .context("Invalid input")?;

    region = fuzzy_match_region(&region);

    let lines_unix = format!(
        "\nexport HUAWEICLOUD_SDK_AK=\"{}\"\nexport HUAWEICLOUD_SDK_SK=\"{}\"\nexport HUAWEICLOUD_SDK_REGION=\"{}\"\n",
        ak, sk, region
    );

    // EXPERIMENTAL Windows PowerShell format
    let lines_windows = format!(
        "\n[Environment]::SetEnvironmentVariable(\"HUAWEICLOUD_SDK_AK\", \"{}\", \"User\")\n\
         [Environment]::SetEnvironmentVariable(\"HUAWEICLOUD_SDK_SK\", \"{}\", \"User\")\n\
         [Environment]::SetEnvironmentVariable(\"HUAWEICLOUD_SDK_REGION\", \"{}\", \"User\")\n",
        ak, sk, region
    );

    if cfg!(windows) {
        info!("Windows detected, writing environment variables");

        // Append to PowerShell profile
        let mut path = dirs::home_dir().ok_or_else(|| anyhow!("Home directory not found"))?;
        path.push("Documents");
        path.push("WindowsPowerShell");
        std::fs::create_dir_all(&path).ok();
        path.push("Microsoft.PowerShell_profile.ps1");

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        file.write_all(lines_windows.as_bytes())?;
    } else {
        info!("Linux detected, writing environment variables");
        for rc in &[".bashrc", ".zshrc"] {
            let mut path = dirs::home_dir().ok_or_else(|| anyhow!("Home directory not found"))?;
            path.push(rc);
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .with_context(|| format!("Failed to open {}", path.display()))?;
            file.write_all(lines_unix.as_bytes())?;
        }
    }

    Ok(())
}

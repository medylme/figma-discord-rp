use std::io::{Write, stdin, stdout};

use indicatif::{ProgressBar, ProgressStyle};

use super::core::{UpdateError, get_releases_url};

fn try_alloc_console() -> bool {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        use windows_sys::Win32::System::Console::AllocConsole;
        unsafe { AllocConsole() != 0 }
    }
    #[cfg(not(all(target_os = "windows", not(debug_assertions))))]
    {
        false
    }
}

#[allow(clippy::needless_return)]
fn free_console_if_owned(owned: bool) {
    if !owned {
        return;
    }
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        use windows_sys::Win32::System::Console::FreeConsole;
        unsafe {
            let _ = FreeConsole();
        }
    }
}

fn prompt_open_release(
    version: &semver::Version,
    tag: &str,
    reason: &str,
) -> Result<(), UpdateError> {
    println!(
        "\n\x1b[33m!\x1b[0m New version v{} found, but {}.",
        version, reason
    );
    print!("Open release page in browser? [Y/n] ");
    let _ = stdout().flush();

    let mut input = String::new();
    if stdin().read_line(&mut input).is_ok() {
        let input = input.trim().to_lowercase();
        if input.is_empty() || input == "y" || input == "yes" {
            if let Some(releases_url) = get_releases_url() {
                let url = format!("{}/{}", releases_url, tag);
                if open::that(&url).is_err() {
                    println!("Failed to open browser. Visit: {}", url);
                }
            } else {
                println!("Release URL could not be determined.");
            }
        }
    }

    Err(UpdateError::UserDeclined)
}

pub fn run_startup_update_check() -> Result<(), UpdateError> {
    let owned_console = try_alloc_console();

    let client = reqwest::blocking::Client::new();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Checking for updates...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let release = match super::core::check_for_updates(&client) {
        Ok(Some(release)) => {
            spinner.finish_and_clear();
            release
        }
        Ok(None) => {
            spinner.finish_and_clear();
            free_console_if_owned(owned_console);
            return Ok(());
        }
        Err(_) => {
            spinner.finish_and_clear();
            free_console_if_owned(owned_console);
            return Ok(());
        }
    };

    let result = perform_update(&client, &release, owned_console);

    free_console_if_owned(owned_console);

    result
}

fn perform_update(
    client: &reqwest::blocking::Client,
    release: &super::core::ReleaseInfo,
    owned_console: bool,
) -> Result<(), UpdateError> {
    let (checksum_url, checksum_name) = match (&release.checksum_url, &release.checksum_name) {
        (Some(url), Some(name)) => (url.clone(), name.clone()),
        _ => {
            return prompt_open_release(
                &release.version,
                &release.tag_name,
                "could not verify signature (no checksum file)",
            );
        }
    };

    let temp_dir = tempfile::tempdir()?;
    let binary_path = temp_dir.path().join(&release.binary_name);
    let checksum_path = temp_dir.path().join(&checksum_name);

    if super::download::download_file(client, &checksum_url, &checksum_path, 0, |_| {}).is_err() {
        return prompt_open_release(
            &release.version,
            &release.tag_name,
            "could not verify signature (failed to download checksum)",
        );
    }

    let checksum_content = match std::fs::read_to_string(&checksum_path) {
        Ok(content) => content,
        Err(_) => {
            return prompt_open_release(
                &release.version,
                &release.tag_name,
                "could not verify signature (failed to read checksum)",
            );
        }
    };

    let expected_hash =
        match super::download::parse_checksum_file(&checksum_content, &release.binary_name) {
            Some(hash) => hash,
            None => {
                return prompt_open_release(
                    &release.version,
                    &release.tag_name,
                    "could not verify signature (invalid checksum format)",
                );
            }
        };

    let pb = ProgressBar::new(release.size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.cyan} [{elapsed_precise}] [{wide_bar:.cyan/white}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(format!(
        "New version available! Downloading v{}...",
        release.version
    ));

    super::download::download_file(
        client,
        &release.binary_url,
        &binary_path,
        release.size,
        |progress| {
            pb.set_position((progress * release.size as f32) as u64);
        },
    )?;

    pb.finish_and_clear();
    println!("\x1b[32m+\x1b[0m Download complete");

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Verifying...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    match super::download::verify_checksum(&binary_path, &expected_hash) {
        Ok(true) => {
            spinner.finish_and_clear();
            println!("\x1b[32m+\x1b[0m Verified");
        }
        Ok(false) | Err(_) => {
            spinner.finish_and_clear();
            println!("\x1b[31mx\x1b[0m Verification failed");
            return prompt_open_release(
                &release.version,
                &release.tag_name,
                "could not verify signature (checksum mismatch)",
            );
        }
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Installing...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    super::install::install_update(&binary_path)?;

    spinner.finish_and_clear();
    println!("\x1b[32m+\x1b[0m Installed");

    println!("\nRestarting...");
    std::thread::sleep(std::time::Duration::from_millis(500));

    free_console_if_owned(owned_console);

    super::install::restart_application()?;

    Ok(())
}

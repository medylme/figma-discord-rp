use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::VERSION;

use super::core::UpdateError;

pub fn download_file<F>(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &Path,
    total_size: u64,
    mut on_progress: F,
) -> Result<(), UpdateError>
where
    F: FnMut(f32),
{
    let mut response = client
        .get(url)
        .header("User-Agent", format!("figma-discord-rp/{}", VERSION))
        .send()?
        .error_for_status()?;

    let mut file = std::fs::File::create(dest)?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;

        if total_size > 0 {
            on_progress(downloaded as f32 / total_size as f32);
        }
    }

    file.flush()?;
    Ok(())
}

pub fn parse_checksum_file(content: &str, filename: &str) -> Option<String> {
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == filename {
            return Some(parts[0].to_lowercase());
        }
    }
    None
}

pub fn calculate_sha256(path: &Path) -> Result<String, UpdateError> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_checksum(file_path: &Path, expected_hash: &str) -> Result<bool, UpdateError> {
    let actual = calculate_sha256(file_path)?;
    Ok(actual.to_lowercase() == expected_hash.to_lowercase())
}

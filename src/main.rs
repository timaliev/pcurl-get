// main.rs
// vim: set ft=rs
//
use anyhow::{Context, Result};
use chrono::{Local, Utc};
use clap::Parser;
use reqwest::{Client, Url};
use sha2::{Digest, Sha256};
use std::{
  path::{Path, PathBuf},
  time::Instant,
};
use tokio::{fs::File, io::AsyncWriteExt, task};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  /// Path to file containing URLs (one per line)
  urls_file: PathBuf,

  /// Save responses to files (named as index-url_hash). If not set, discard content.
  #[arg(
    short,
    long,
    help = "Save responses to files (named as index-url_hash)"
  )]
  save: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  let urls = read_urls(&args.urls_file).context("Failed to read URLs file")?;
  let mut handles = Vec::with_capacity(urls.len());

  // Record start time of request loop
  let start_instant = Instant::now();
  let offset = Local::now().signed_duration_since(Utc::now()).num_hours();

  println!(
    "🚀 Started at: {} T{:?}:00",
    Local::now().format("%Y-%m-%d %H:%M:%S"),
    offset,
  );

  let client = Client::new();
  for (i, url) in urls.iter().enumerate() {
    let client = client.clone();
    let url = url.trim().to_string();
    let save = args.save;
    eprintln!("DEBUG: fetching {} {}", i, url);
    let handle = task::spawn(async move {
      fetch_url(&client, &url, save, i)
        .await
        .context(format!("Failed to fetch {}", url))
    });
    handles.push(handle);
  }

  let mut results = Vec::with_capacity(handles.len());
  for handle in handles {
    results.push(handle.await?);
  }
  println!("✅ Successfully processed {} URLs", results.len());
  // println!("Received {:?} bytes of content", );
  // Calculate and display duration
  let duration = start_instant.elapsed().as_secs_f64();
  println!(
    "⏱️  Finished at: {} T{:?}:00",
    Local::now().format("%Y-%m-%d %H:%M:%S"),
    offset,
  );
  println!("⏱️  Duration: {:.6} seconds", duration);

  Ok(())
}

pub fn read_urls(path: &Path) -> Result<Vec<String>> {
  std::fs::read_to_string(path)
    .context("Failed to read file")?
    .lines()
    .map(|line| Ok(line.to_string()))
    .collect()
}

pub fn convert_url(url: &str) -> String {
  match Url::parse(url) {
    Ok(u) => format!(
      "{}-{}{}",
      u.host_str().unwrap_or("unknown").replace(".", "_"),
      u.port_or_known_default().unwrap_or_default(),
      u.path().replace("/", "_"),
    ),
    Err(_) => "unknown".to_string(),
  }
}

pub async fn fetch_url(client: &Client, url: &str, save: bool, index: usize) -> Result<()> {
  let response = client
    .get(url)
    .send()
    .await
    .with_context(|| format!("HTTP request {} failed", index))?;

  let status = response.status();
  println!("GET {} -> {}", url, status);

  if !status.is_success() {
    anyhow::bail!("HTTP {} for {}", status, url);
  }

  let content = response
    .bytes()
    .await
    .context("Failed to read response body")?;

  if save {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let filename = format!(
      "{}-{}-{}",
      index,
      convert_url(url),
      hex::encode(hasher.finalize())
    );
    let mut file = File::create(&filename)
      .await
      .context("Failed to create file")?;
    file
      .write_all(&content)
      .await
      .context("Failed to write file")?;
    println!("  Saved {} bytes to {}", content.len(), filename);
  }
  else {
    // Discard content (like /dev/null)
    println!(" Discarded {} bytes", content.len());
  }

  Ok(())
}

#[cfg(test)]
pub mod tests {
  // See ../tests/tests.rs for unit tests
}

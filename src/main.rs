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

  /// Limit parallel URL requests. Defaults to unlimited (all at once).
  /// If greater than total URLs, requests all URLs in one take.
  #[arg(
    short = 'P',
    long,
    default_value_t = usize::MAX,
    help = "Limit parallel URL requests (default: unlimited)"
  )]
  parallelism: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  let urls = read_urls(&args.urls_file).context("Failed to read URLs file")?;

  // Record start time of request loop
  let start_instant = Instant::now();
  let offset = Local::now().signed_duration_since(Utc::now()).num_hours();

  println!(
    "🚀 Started at: {} T{:?}:00",
    Local::now().format("%Y-%m-%d %H:%M:%S"),
    offset,
  );

  let parallelism = if args.parallelism > urls.len() {
    urls.len()
  } else {
    args.parallelism
  };
  println!("🔧 Parallelism: {} concurrent requests", parallelism);

  let client = Client::new();
  let mut results = Vec::with_capacity(urls.len());

  for chunk in urls.chunks(parallelism) {
    let mut batch = Vec::with_capacity(chunk.len());
    for (i, url) in chunk.iter().enumerate() {
      let client = client.clone();
      let url = url.trim().to_string();
      let save = args.save;
      let global_index = i + results.len();
      let handle = task::spawn(async move {
        fetch_url(&client, &url, save, global_index)
          .await
          .context(format!("Failed to fetch {}", url))
      });
      batch.push(handle);
    }
    for handle in batch {
      results.push(handle.await?);
    }
  }
  println!(
    "✅ Successfully processed {} lines in {}",
    results.len(),
    &args.urls_file.to_string_lossy()
  );
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
    .filter_map(|line| Url::parse(line).ok())
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

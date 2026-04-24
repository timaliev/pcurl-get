// main.rs
// vim: set ft=rs
//
use anyhow::{Context, Result};
use clap::Parser;
use reqwest::{Client, Url};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::{fs::File, io::AsyncWriteExt, task};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
  /// Path to file containing URLs (one per line)
  urls_file: PathBuf,

  /// Save responses to files (named as url_hash.txt). If not set, discard content.
  #[arg(short, long)]
  save: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  let urls = read_urls(&args.urls_file).context("Failed to read URLs file")?;

  let client = Client::new();
  let mut handles = Vec::with_capacity(urls.len());

  for url in urls {
    let client = client.clone();
    let url = url.trim().to_string();
    let save = args.save;

    let handle = task::spawn(async move {
      fetch_url(&client, &url, save)
        .await
        .context(format!("Failed to fetch {}", url))
    });
    handles.push(handle);
  }

  let mut results = Vec::with_capacity(handles.len());
  for handle in handles {
    results.push(handle.await?);
  }
  println!("Successfully processed {} URLs", results.len());
  Ok(())
}

fn read_urls(path: &Path) -> Result<Vec<String>> {
  std::fs::read_to_string(path)
    .context("Failed to read file")?
    .lines()
    .map(|line| Ok(line.to_string()))
    .collect()
}

fn convert_url(url: &str) -> String {
  match Url::parse(url) {
    Ok(u) => format!(
      "{}-{}-{}",
      u.host_str().unwrap_or("unknown").replace(".", "_"),
      u.port_or_known_default().unwrap_or_default(),
      u.path().replace("/", "_"),
    ),
    Err(_) => "unknown".to_string(),
  }
}

async fn fetch_url(client: &Client, url: &str, save: bool) -> Result<()> {
  let response = client
    .get(url)
    .send()
    .await
    .context("HTTP request failed")?;

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
      "{}-{}.txt",
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

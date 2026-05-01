// tests/tests.rs
// vim: set ft=rs
//
//! Comprehensive unit and integration tests for pcurl-get
//!
//! This file tests:
//! - `read_urls()`: File reading and URL parsing
//! - `convert_url()`: URL to filename conversion
//! - `fetch_url()`: HTTP fetching with mockito
//! - `main()`: Integration tests with real HTTP scenarios
//!
//! Run tests with: cargo test

use anyhow::{Result, bail};
use mockito::Mock;
use reqwest::Client;
use std::{io::Write, path::PathBuf, str::FromStr};
use tempfile::{NamedTempFile, tempdir};
use tokio::{fs, fs::File, io::AsyncReadExt};

#[path = "../src/main.rs"]
mod pcurl_get;
use crate::pcurl_get::*;

// ===========================
// Tests for read_urls
// ===========================

#[test]
fn read_urls_empty_file() -> Result<()> {
  let file = NamedTempFile::new().unwrap();
  std::fs::write(file.path(), "")?;
  let path = file.path().to_path_buf();

  let urls = pcurl_get::read_urls(&path)?;
  assert!(urls.is_empty());

  Ok(())
}

#[test]
fn read_urls_single_url() -> Result<()> {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "https://example.com")?;
  let path = file.path().to_path_buf();

  let urls = read_urls(&path)?;
  assert_eq!(urls.len(), 1);
  assert_eq!(urls[0], "https://example.com");

  Ok(())
}

#[test]
fn read_urls_multiple_urls() -> Result<()> {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "https://example.com")?;
  writeln!(file, "https://test.org/path")?;
  writeln!(file, "https://api.github.com")?;
  let path = file.path().to_path_buf();

  let urls = read_urls(&path)?;
  assert_eq!(urls.len(), 3);
  assert_eq!(urls[0], "https://example.com");
  assert_eq!(urls[1], "https://test.org/path");
  assert_eq!(urls[2], "https://api.github.com");

  Ok(())
}

#[test]
fn read_urls_with_empty_lines() -> Result<()> {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "https://example.com")?;
  writeln!(file)?;
  writeln!(file, "https://test.org")?;
  writeln!(file, "   ")?; // spaces only
  let path = file.path().to_path_buf();

  let urls = read_urls(&path)?;
  assert_eq!(urls.len(), 4); // empty lines and whitespace preserved

  Ok(())
}

#[test]
fn read_urls_with_trailing_newlines() -> Result<()> {
  let mut file = NamedTempFile::new().unwrap();
  writeln!(file, "https://example.com")?;
  writeln!(file, "https://test.org")?;
  // No final newline - test still reads correctly
  let path = file.path().to_path_buf();

  let urls = read_urls(&path)?;
  assert_eq!(urls.len(), 2);

  Ok(())
}

#[test]
fn read_urls_nonexistent_file() -> Result<()> {
  let path = PathBuf::from_str("/nonexistent/path/to/urls.txt")?;
  let result = read_urls(&path);
  let err = result.expect_err("Expected error for nonexistent file");
  assert!(err.to_string().contains("Failed to read file"));

  Ok(())
}

// ===========================
// Tests for convert_url
// ===========================

#[test]
fn convert_url_simple() {
  let url = "https://example.com/path/";
  let converted = convert_url(url);
  assert_eq!(converted, "example_com-443_path_");
}

#[test]
fn convert_url_with_different_ports() {
  let url = "http://localhost:8080/api/v1";
  let converted = convert_url(url);
  assert!(converted.contains("localhost-8080_api_v1"));
}

#[test]
fn convert_url_with_subdomains() {
  let url = "https://sub.domain.example.com/path/to/resource";
  let converted = convert_url(url);
  assert!(converted.contains("sub_domain_example_com-443_path_to_resource"));
}

#[test]
fn convert_url_with_query_params() {
  let url = "https://example.com/search?q=hello&foo=bar";
  let converted = convert_url(url);
  assert!(converted.contains("example_com-443_search"));
}

#[test]
fn convert_url_ip_address() {
  let url = "https://192.168.1.1/admin";
  let converted = convert_url(url);
  assert!(converted.contains("192_168_1_1-443_admin"));
}

#[test]
fn convert_url_invalid() {
  let url = "not-a-url";
  let converted = convert_url(url);
  assert_eq!(converted, "unknown");
}

#[test]
fn convert_url_file_protocol() {
  let url = "file:///tmp/test.txt";
  let converted = convert_url(url);
  assert!(converted.contains("unknown-"));
}

#[test]
fn convert_url_http_no_port() {
  let url = "http://example.com";
  let converted = convert_url(url);
  // HTTP default is 80
  assert!(converted.contains("example_com-80_"));
}

#[test]
fn convert_url_https_no_port() {
  let url = "https://example.com";
  let converted = convert_url(url);
  // HTTPS default is 443
  assert!(converted.contains("example_com-443_"));
}

#[test]
fn convert_url_empty_string() {
  let url = "";
  let converted = convert_url(url);
  assert_eq!(converted, "unknown");
}

#[test]
fn convert_url_no_path() {
  let url = "https://example.com";
  let converted = convert_url(url);
  assert!(converted.contains("example_com-443_"));
}

#[test]
fn convert_url_with_fragment() {
  let url = "https://example.com/page#section";
  let converted = convert_url(url);
  assert!(converted.contains("example_com-443_page"));
}

#[test]
fn convert_url_with_credentials() {
  let url = "https://user:pass@example.com/path";
  let converted = convert_url(url);
  assert!(converted.contains("example_com-443_path"));
}

// =================================================
// Tests for fetch_url (with mockito)
// =================================================

#[tokio::test]
async fn fetch_url_success() -> Result<()> {
  use mockito::Server;

  let mut server = Server::new_async().await;
  let path = "/test";
  let url = format!("{}{}", server.url(), path);
  let mock = server
    .mock("GET", "/test")
    .with_body("test content")
    .create_async()
    .await;

  let client = Client::builder().build()?;

  let result = fetch_url(&client, &url, false, 0).await;
  assert!(result.is_ok());

  // Verify mock was called
  assert!(mock.matched());

  mock.assert_async().await;
  Ok(())
}

#[tokio::test]
async fn fetch_url_404_error() -> Result<()> {
  use mockito::Server;

  let mut server = Server::new_async().await;
  let path = "/notfound";
  let url = format!("{}{}", server.url(), path);
  let mock = server
    .mock("GET", "/notfound")
    .with_status(404)
    .with_body("Not Found")
    .create_async()
    .await;

  let client = Client::builder().build()?;
  let result = fetch_url(&client, url.as_str(), false, 0).await;

  // Should fail with 404 error
  let err = result.expect_err("Expected HTTP error for 404");
  assert!(err.to_string().contains("404"));

  mock.assert_async().await;
  Ok(())
}

#[tokio::test]
async fn fetch_url_500_error() -> Result<()> {
  use mockito::Server;

  let mut server = Server::new_async().await;
  let path = "/server-error";
  let url = format!("{}{}", server.url(), path);
  let mock = server
    .mock("GET", "/server-error")
    .with_status(500)
    .with_body("Internal Server Error")
    .create_async()
    .await;

  let client = Client::builder().build()?;

  let result = fetch_url(&client, url.as_str(), false, 0).await;
  let err = result.expect_err("Expected HTTP error for 500");
  assert!(err.to_string().contains("500"));

  mock.assert_async().await;
  Ok(())
}

#[tokio::test]
async fn fetch_url_403_error() -> Result<()> {
  use mockito::Server;

  let mut server = Server::new_async().await;
  let path = "/forbidden";
  let url = format!("{}{}", server.url(), path);
  let mock = server
    .mock("GET", "/forbidden")
    .with_status(403)
    .with_body("Forbidden")
    .create_async()
    .await;

  let client = Client::builder().build()?;

  let result = fetch_url(&client, url.as_str(), false, 0).await;
  let err = result.expect_err("Expected HTTP error for 403");
  assert!(err.to_string().contains("403"));

  mock.assert_async().await;
  Ok(())
}

#[tokio::test]
async fn fetch_url_large_content() -> Result<()> {
  const CONTENT_SIZE: usize = 1024 * 1024; // 1MB

  let mut server = mockito::Server::new_async().await;
  let path = "/large";
  let url = format!("{}{}", server.url(), path);
  let large_content = vec![b'A'; CONTENT_SIZE];
  let mock = server
    .mock("GET", "/large")
    .with_body(&large_content)
    .create_async()
    .await;

  let client = Client::builder().build()?;

  let result = fetch_url(&client, url.as_str(), false, 0).await;
  assert!(result.is_ok());
  mock.assert_async().await;

  Ok(())
}

#[tokio::test]
async fn fetch_url_content_length() -> Result<()> {
  use sha2::{Digest, Sha256};
  const EXPECTED_CONTENT: &[u8] = b"test content exactly";

  let mut server = mockito::Server::new_async().await;
  let path = "/content-check";
  let url = format!("{}{}", server.url(), path);
  let mock = server
    .mock("GET", "/content-check")
    .with_body(EXPECTED_CONTENT)
    .create_async()
    .await;

  let client = Client::builder().build()?;

  let result = fetch_url(&client, url.as_str(), true, 0).await;
  assert!(result.is_ok());

  let mut hasher = Sha256::new();
  hasher.update(url.as_bytes());
  let filename = format!(
    "{}-{}-{}",
    0,
    convert_url(&url),
    hex::encode(hasher.finalize())
  );
  let path = PathBuf::from_str(filename.as_str())?;
  let mut file = File::open(&path).await?;
  let mut contents = Vec::new();
  let _ = file.read_to_end(&mut contents).await;
  assert_eq!(contents, EXPECTED_CONTENT);
  // Unlink file
  fs::remove_file(path).await?;
  mock.assert_async().await;

  Ok(())
}

#[tokio::test]
async fn fetch_url_concurrent_requests() -> Result<()> {
  use mockito::Server;
  const ENDPOINTSNUM: u8 = 5;
  let mut server = Server::new_async().await;
  let path = "/concurrent";

  // Create ENDPOINTSNUM mock endpoints
  let mut mocks: Vec<Mock> = Vec::with_capacity(ENDPOINTSNUM.into());
  for i in 0..ENDPOINTSNUM {
    let mock = server
      .mock("GET", format!("{path}{i}").as_str())
      .with_body(format!("content{i}").as_bytes())
      .create_async()
      .await;
    mocks.push(mock);
  };

  let client = Client::builder().build()?;

  // Create 3 concurrent requests
  let mut handles = Vec::new();
  for i in 0..ENDPOINTSNUM {
    let client = client.clone();
    let url = format!("{}{path}{i}", server.url());
    handles.push(tokio::spawn(async move {
      fetch_url(&client, &url, false, 0).await
    }));
  }

  // Wait for all concurrent requests
  for handle in handles {
    let result = handle.await;
    assert!(result.is_ok(), "Concurrent request failed");
  }
  for mock in mocks {
    mock.assert_async().await;
  }

  Ok(())
}

// ===========================
// Tests for sha2 hashing
// ===========================

#[test]
fn sha256_hashing() {
  use sha2::{Digest, Sha256};

  let mut hasher = Sha256::new();
  hasher.update(b"test");
  let result = hex::encode(hasher.finalize());

  assert_eq!(result.len(), 64); // SHA256 produces 64 hex characters
}

#[test]
fn sha256_consistency() {
  use sha2::{Digest, Sha256};

  let input = b"consistent hash test";
  let mut hasher = Sha256::new();
  hasher.update(input);
  let hash1 = hex::encode(hasher.finalize());

  let mut hasher = Sha256::new();
  hasher.update(input);
  let hash2 = hex::encode(hasher.finalize());

  assert_eq!(hash1, hash2);
}

// ============================================================================
// Integration tests for main
// ============================================================================

#[tokio::test]
async fn main_with_empty_urls_file() -> Result<()> {
  use tempfile::tempdir;

  let dir = tempdir()?;
  let urls_file = dir.path().join("urls.txt");
  std::fs::write(&urls_file, "")?;

  // Create a simple client that won't make actual requests
  let empty: Vec<String> = Vec::new();
  // For empty file, we should succeed
  let result = read_urls(&urls_file)?;
  assert_eq!(result, empty);

  Ok(())
}

#[tokio::test]
async fn main_with_single_url() -> Result<()> {
  use mockito::Server;

  let mut mock_server = Server::new_async().await;
  let path = "/single";
  let url = format!("{}{}", mock_server.url(), path);
  let mock = mock_server
    .mock("GET", path)
    .with_body(b"single url test")
    .create_async()
    .await;

  let dir = tempdir()?;
  let urls_file = dir.path().join("urls.txt");
  std::fs::write(&urls_file, url)?;

  let urls = read_urls(&urls_file)?;

  let client = Client::builder().build()?;
  for (i, url) in urls.iter().enumerate() {
    let client = client.clone();
    let result = fetch_url(&client, url.as_str(), false, i).await;
    match result {
      Ok(_) => {
        // Verify mock was called
        assert!(mock.matched());
        // assert_eq!(r, vec!["single url test"]);
      }
      Err(e) => {
        bail!("main failed: {:?}", e);
        // This might fail due to CLI argument handling, which is expected
      }
    }
  }
  mock.assert_async().await;

  Ok(())
}

#[tokio::test]
async fn main_with_multiple_urls() -> Result<()> {
  use mockito::Server;

  let mut mock_server = Server::new_async().await;

  let path1 = "/url1";
  let path2 = "/url2";
  let mock1 = mock_server
    .mock("GET", path1)
    .with_body(b"content1")
    .create_async()
    .await;
  let mock2 = mock_server
    .mock("GET", path2)
    .with_body(b"content2")
    .create_async()
    .await;

  let dir = tempdir()?;
  let urls_file = dir.path().join("urls.txt");
  let urls_to_write = format!(
    "{}{}\n{}{}\n",
    mock_server.url(),
    path1,
    mock_server.url(),
    path2
  );
  std::fs::write(&urls_file, urls_to_write)?;

  let urls = read_urls(&urls_file)?;
  for (i, url) in urls.iter().enumerate() {
    let client = Client::builder().build()?;
    let result = fetch_url(&client, url.as_str(), false, i).await;
    match result {
      Ok(_) => {
        mock1.matched();
        mock2.matched();
      }
      Err(e) => {
        bail!("main failed: {:?}", e);
      }
    }
  }
  mock1.assert_async().await;
  mock2.assert_async().await;

  Ok(())
}

#[tokio::test]
async fn main_with_save_option() -> Result<()> {
  use sha2::{Digest, Sha256};
  use tempfile::tempdir;
  // use std::fs;

  const EXPECTED_CONTENT: &[u8] = b"test content for saving";
  let mut mock_server = mockito::Server::new_async().await;
  let path = "/save-test";
  let url = format!("{}{}", mock_server.url(), path);
  let mock = mock_server
    .mock("GET", path)
    .with_body(EXPECTED_CONTENT)
    .create_async()
    .await;

  let dir = tempdir()?;
  let urls_file = dir.path().join("urls.txt");
  std::fs::write(&urls_file, &url)?;

  let urls = read_urls(&urls_file)?;
  let client = Client::builder().build()?;

  for (i, url) in urls.iter().enumerate() {
    let client = client.clone();
    let result = fetch_url(&client, url.as_str(), true, i).await;
    assert!(result.is_ok());
    match result {
      Ok(()) => {
        mock.assert_async().await;
      }
      Err(e) => {
        bail!("main failed with save=true: {:?}", e);
      }
    }
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let filename = format!(
      "{}-{}-{}",
      0,
      convert_url(url),
      hex::encode(hasher.finalize())
    );
    let path = PathBuf::from_str(filename.as_str())?;
    // eprintln!("File path to read: {}", path.to_string_lossy());
    let mut file = File::open(&path).await?;
    let mut contents = Vec::new();
    let _ = file.read_to_end(&mut contents).await;
    assert_eq!(contents, EXPECTED_CONTENT);
    fs::remove_file(path).await?;
  }

  Ok(())
}

#[tokio::test]
async fn main_with_whitespace_only_urls() -> Result<()> {
  use tempfile::tempdir;

  let dir = tempdir()?;
  let urls_file = dir.path().join("urls.txt");
  std::fs::write(&urls_file, "   \n\n   \nhttps://example.com\n")?;

  // This should succeed (whitespace-only URLs are not filtered out)
  let result = read_urls(&urls_file);
  match result {
    Ok(urls) => {
      assert_eq!(urls.len(), 4); // no validity is tested on file read
      Ok(())
    }
    Err(e) => {
      bail!("main failed with whitespace URLs: {:?}", e);
      // This might fail if there are no valid URLs after filtering
    }
  }
}

// ===========================
// End of tests
// ===========================

//! HuggingFace Hub API helpers.
//!
//! Covers:
//!  - Parsing a repo identifier from a bare `owner/repo` string or a full URL.
//!  - Listing GGUF siblings via the Hub API.
//!  - Downloading a single file with a streaming progress bar.
//!  - Fetching the raw README / model-card text.

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use serde::Deserialize;

// ── Constants ─────────────────────────────────────────────────────────────────

const HF_BASE: &str = "https://huggingface.co";
const HF_API: &str = "https://huggingface.co/api/models";
const USER_AGENT: &str = concat!("gguf-analyzer/", env!("CARGO_PKG_VERSION"));

// ── Repo identifier ───────────────────────────────────────────────────────────

/// A resolved `owner/repo` pair (e.g. `Qwen/Qwen3-0.6B-GGUF`).
#[derive(Debug, Clone)]
pub struct RepoId {
    pub owner: String,
    pub repo: String,
}

impl RepoId {
    /// Parse from `owner/repo` or any `https://huggingface.co/owner/repo[/…]` URL.
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let trimmed = input.trim_end_matches('/');

        // Strip protocol + host if present.
        let path = if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
            let without_scheme = trimmed
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            // Drop the host (huggingface.co or anything else).
            without_scheme
                .split_once('/')
                .map(|(_, rest)| rest)
                .unwrap_or(without_scheme)
        } else {
            trimmed
        };

        // We only need the first two path segments (owner/repo).
        let mut parts = path.splitn(3, '/');
        let owner = parts
            .next()
            .filter(|s| !s.is_empty())
            .context("cannot parse repo owner from input")?;
        let repo = parts
            .next()
            .filter(|s| !s.is_empty())
            .context("cannot parse repo name from input")?;

        Ok(Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }

    /// `owner/repo` string.
    pub fn id(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }

    /// URL to the Hub API metadata endpoint.
    pub fn api_url(&self) -> String {
        format!("{}/{}/{}", HF_API, self.owner, self.repo)
    }

    /// URL for a specific file in the default branch.
    pub fn file_url(&self, filename: &str) -> String {
        format!(
            "{}/{}/{}/resolve/main/{}",
            HF_BASE, self.owner, self.repo, filename
        )
    }

    /// URL to the raw README.
    pub fn readme_url(&self) -> String {
        self.file_url("README.md")
    }
}

// ── API types ─────────────────────────────────────────────────────────────────

/// A single file entry returned by the Hub API `siblings` list.
#[derive(Debug, Clone, Deserialize)]
pub struct Sibling {
    pub rfilename: String,
    pub size: Option<u64>,
}

/// Minimal subset of the Hub API model metadata response.
#[derive(Debug, Deserialize)]
pub struct HfModelInfo {
    pub siblings: Vec<Sibling>,
}

// ── Client ────────────────────────────────────────────────────────────────────

/// Build a reusable `reqwest` blocking client with a sensible user-agent.
pub fn client() -> anyhow::Result<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("failed to build HTTP client")
}

// ── Listing ───────────────────────────────────────────────────────────────────

/// Return all `.gguf` siblings for `repo`.
pub fn list_gguf_files(client: &Client, repo: &RepoId) -> anyhow::Result<Vec<Sibling>> {
    let url = repo.api_url();
    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("GET {url}"))?;

    if !resp.status().is_success() {
        bail!(
            "Hub API returned {} for repo '{}' — does the repo exist and is it public?",
            resp.status(),
            repo.id()
        );
    }

    let info: HfModelInfo = resp.json().context("failed to parse Hub API response")?;
    let ggufs: Vec<_> = info
        .siblings
        .into_iter()
        .filter(|s| s.rfilename.ends_with(".gguf"))
        .collect();

    Ok(ggufs)
}

// ── Download ──────────────────────────────────────────────────────────────────

/// Download `url` into `dest`, showing a progress bar.
///
/// If `dest` is a directory the filename is inferred from the URL path.
pub fn download(client: &Client, url: &str, dest: &Path, force: bool) -> anyhow::Result<PathBuf> {
    // `dest` is always treated as a directory — create it if it doesn't exist,
    // then append the filename derived from the URL.
    fs::create_dir_all(dest).with_context(|| format!("create directory '{}'", dest.display()))?;

    let filename = url
        .rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .context("cannot infer filename from URL")?;

    let out_path = dest.join(filename);

    if out_path.exists() && !force {
        bail!(
            "output file '{}' already exists — pass --force to overwrite",
            out_path.display()
        );
    }

    // Send request.
    let resp = client
        .get(url)
        .send()
        .with_context(|| format!("GET {url}"))?;

    if !resp.status().is_success() {
        bail!("server returned {} for {url}", resp.status());
    }

    let total = resp.content_length();

    // Set up progress bar.
    let pb = ProgressBar::new(total.unwrap_or(0));
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("=>-"),
    );
    pb.set_message(
        out_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );

    // Stream to file.
    let mut file =
        fs::File::create(&out_path).with_context(|| format!("create '{}'", out_path.display()))?;

    let mut downloaded = 0u64;
    let mut buf = Vec::with_capacity(128 * 1024);

    use std::io::Read as _;
    let mut body = resp;
    let mut chunk = [0u8; 65536];
    loop {
        let n = body
            .read(&mut chunk)
            .context("read error during download")?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() >= 1024 * 1024 {
            file.write_all(&buf)
                .context("write error during download")?;
            downloaded += buf.len() as u64;
            pb.set_position(downloaded);
            buf.clear();
        }
    }
    if !buf.is_empty() {
        file.write_all(&buf)
            .context("write error during download")?;
        downloaded += buf.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!(
        "✓ {}",
        out_path.file_name().unwrap_or_default().to_string_lossy()
    ));
    Ok(out_path)
}

// ── README ────────────────────────────────────────────────────────────────────

/// Fetch the raw README text for a repo.  Returns `None` if no README exists.
pub fn fetch_readme(client: &Client, repo: &RepoId) -> anyhow::Result<Option<String>> {
    let url = repo.readme_url();
    let resp = client
        .get(&url)
        .send()
        .with_context(|| format!("GET {url}"))?;

    if resp.status().as_u16() == 404 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        bail!("server returned {} fetching README", resp.status());
    }

    Ok(Some(resp.text().context("read README text")?))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_repo_id() {
        let r = RepoId::parse("Qwen/Qwen3-0.6B-GGUF").unwrap();
        assert_eq!(r.owner, "Qwen");
        assert_eq!(r.repo, "Qwen3-0.6B-GGUF");
    }

    #[test]
    fn parse_full_url() {
        let r = RepoId::parse("https://huggingface.co/Qwen/Qwen3-0.6B-GGUF").unwrap();
        assert_eq!(r.owner, "Qwen");
        assert_eq!(r.repo, "Qwen3-0.6B-GGUF");
    }

    #[test]
    fn parse_url_with_trailing_slash() {
        let r = RepoId::parse("https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/").unwrap();
        assert_eq!(r.repo, "Qwen3-0.6B-GGUF");
    }

    #[test]
    fn parse_url_with_extra_segments() {
        let r = RepoId::parse("https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/tree/main").unwrap();
        assert_eq!(r.owner, "Qwen");
        assert_eq!(r.repo, "Qwen3-0.6B-GGUF");
    }

    #[test]
    fn file_url_is_correct() {
        let r = RepoId::parse("Qwen/Qwen3-0.6B-GGUF").unwrap();
        assert_eq!(
            r.file_url("Qwen3-0.6B-Q8_0.gguf"),
            "https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf"
        );
    }

    #[test]
    fn parse_missing_owner_errors() {
        assert!(RepoId::parse("").is_err());
        assert!(RepoId::parse("onlyone").is_err());
    }
}

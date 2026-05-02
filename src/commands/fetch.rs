//! `fetch` subcommand — list and download GGUF files from HuggingFace.
//!
//! # Examples
//!
//! ```text
//! # List available quantisations
//! gguf-analyzer fetch Qwen/Qwen3-0.6B-GGUF --list
//!
//! # Download the Q8_0 quant to the current directory
//! gguf-analyzer fetch Qwen/Qwen3-0.6B-GGUF --file Q8_0
//!
//! # Download a direct URL
//! gguf-analyzer fetch https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/main/Qwen3-0.6B-Q8_0.gguf
//! ```

use anyhow::{Context as _, bail};
use colored::Colorize as _;
use glob::Pattern;
use tabled::{Table, Tabled, settings::Style};

use crate::{
    cli::FetchArgs,
    hf::{self, RepoId},
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(args: &FetchArgs) -> anyhow::Result<()> {
    let client = hf::client()?;

    // Detect if the input is a direct GGUF URL (no --file needed).
    if is_direct_url(&args.repo) {
        return download_direct(&client, args);
    }

    let repo = RepoId::parse(&args.repo)
        .with_context(|| format!("cannot parse repo from '{}'", args.repo))?;

    // Fetch the list of GGUF files.
    eprintln!("{} {} …", "Querying".cyan().bold(), repo.id().bold());
    let files = hf::list_gguf_files(&client, &repo)?;

    if files.is_empty() {
        bail!("no GGUF files found in repo '{}'", repo.id());
    }

    // --list: just print the table and exit.
    if args.list {
        print_files_table(&files);
        return Ok(());
    }

    // Resolve which file to download.
    let target = match &args.file {
        Some(pattern) => {
            let pat = Pattern::new(&format!("*{}*", pattern.to_lowercase()))
                .with_context(|| format!("invalid glob pattern '{pattern}'"))?;
            let matches: Vec<_> = files
                .iter()
                .filter(|f| pat.matches(&f.rfilename.to_lowercase()))
                .collect();
            match matches.len() {
                0 => {
                    eprintln!("\nAvailable files:");
                    print_files_table(&files);
                    bail!("no GGUF file matches pattern '{pattern}'");
                }
                1 => matches[0].clone(),
                _ => {
                    eprintln!(
                        "{} multiple files match '{pattern}' — pick one:",
                        "Ambiguous:".yellow().bold()
                    );
                    print_files_table(&matches.iter().map(|f| (*f).clone()).collect::<Vec<_>>());
                    bail!("refine your --file pattern");
                }
            }
        }
        None => {
            // No pattern given — print the list and ask the user to pick.
            eprintln!("\nAvailable GGUF files in {}:", repo.id().bold());
            print_files_table(&files);
            eprintln!(
                "\n{} re-run with {} to download one of the files above.",
                "Hint:".cyan().bold(),
                "--file <PATTERN>".bold()
            );
            return Ok(());
        }
    };

    let url = repo.file_url(&target.rfilename);
    do_download(&client, &url, &target.rfilename, args)?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn is_direct_url(s: &str) -> bool {
    (s.starts_with("https://") || s.starts_with("http://")) && s.ends_with(".gguf")
}

fn download_direct(client: &reqwest::blocking::Client, args: &FetchArgs) -> anyhow::Result<()> {
    let filename = args
        .repo
        .rsplit('/')
        .next()
        .unwrap_or("model.gguf")
        .to_string();
    do_download(client, &args.repo, &filename, args)
}

fn do_download(
    client: &reqwest::blocking::Client,
    url: &str,
    filename: &str,
    args: &FetchArgs,
) -> anyhow::Result<()> {
    let dest = args
        .output_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));

    eprintln!("{} {}", "Downloading".cyan().bold(), filename.bold());
    eprintln!("  URL : {}", url.dimmed());
    eprintln!("  Into: {}", dest.display().to_string().dimmed());

    let saved = hf::download(client, url, &dest, args.force)?;

    eprintln!("{} '{}'", "Saved:".green().bold(), saved.display());

    // Print size.
    if let Ok(meta) = std::fs::metadata(&saved) {
        let mb = meta.len() as f64 / 1_048_576.0;
        eprintln!("  Size: {:.1} MB", mb);
    }
    Ok(())
}

// ── Table display ─────────────────────────────────────────────────────────────

#[derive(Tabled)]
struct FileRow {
    #[tabled(rename = "Filename")]
    filename: String,
    #[tabled(rename = "Size")]
    size: String,
}

fn format_size(bytes: Option<u64>) -> String {
    match bytes {
        None => "—".to_string(),
        Some(b) if b >= 1_073_741_824 => format!("{:.2} GB", b as f64 / 1_073_741_824.0),
        Some(b) if b >= 1_048_576 => format!("{:.1} MB", b as f64 / 1_048_576.0),
        Some(b) => format!("{} B", b),
    }
}

fn print_files_table(files: &[hf::Sibling]) {
    let rows: Vec<FileRow> = files
        .iter()
        .map(|f| FileRow {
            filename: f.rfilename.clone(),
            size: format_size(f.size),
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded());
    println!("{table}");
}

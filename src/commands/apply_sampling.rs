//! `apply-sampling` subcommand — write recommended sampling parameters into a
//! GGUF file's metadata.
//!
//! **Flow:**
//! 1. If `--repo` is given, fetch the HuggingFace README and parse it for
//!    [`SamplingSet`] entries.
//! 2. If multiple sets are found, `--mode <LABEL>` selects one; otherwise the
//!    first set is used.
//! 3. If no sampling parameters are found (or no `--repo` is given), the user
//!    is prompted interactively for each parameter; blank input skips that key.
//! 4. The resolved parameters are written into a new GGUF file at `--output`,
//!    preserving all other metadata and tensor data unchanged.

use anyhow::Context as _;
use colored::Colorize as _;
use std::{
    collections::HashMap,
    fs,
    io::{self},
};
use tracing::instrument;

use gguf_rs_lib::format::metadata::MetadataValue;

use crate::{
    cli::ApplySamplingArgs,
    commands::model_card::{ModelCard, SamplingSet},
    error::AppError,
    gguf::{ParsedGguf, backup_if_exists, write_modified_gguf},
    hf,
    output::resolve_output,
};

// ── GGUF metadata key names for sampling parameters ──────────────────────────
//
// These follow the `llama.cpp` / GGUF spec convention for sampling hint
// metadata keys. Tools that respect these keys (e.g. llama.cpp frontends)
// will pick them up automatically when loading the model file.

const KEY_TEMPERATURE: &str = "llama.sampling.temperature";
const KEY_TOP_P: &str = "llama.sampling.top_p";
const KEY_TOP_K: &str = "llama.sampling.top_k";
const KEY_MIN_P: &str = "llama.sampling.min_p";
const KEY_PRESENCE_PENALTY: &str = "llama.sampling.presence_penalty";
const KEY_REPETITION_PENALTY: &str = "llama.sampling.repetition_penalty";

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the `apply-sampling` subcommand.
#[instrument(skip_all, fields(file = %args.file.display()))]
pub fn run(args: &ApplySamplingArgs) -> anyhow::Result<()> {
    // ── Load GGUF ─────────────────────────────────────────────────────────────
    let mut gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    let output = resolve_output(&args.file, args.output.as_deref(), "-sampled");

    // ── Resolve sampling params ───────────────────────────────────────────────
    let params: HashMap<&'static str, MetadataValue> = if let Some(repo) = &args.repo {
        resolve_from_repo(repo, args.mode.as_deref())?
    } else {
        prompt_interactive()?
    };

    if params.is_empty() {
        println!(
            "{}",
            "No sampling parameters provided — nothing to do.".yellow()
        );
        return Ok(());
    }

    // ── Dry-run output ────────────────────────────────────────────────────────
    if args.dry_run {
        println!("{}", "Sampling parameters that would be written:".bold());
        print_params_table(&params, &gguf);
        println!("  Output file : {}", output.display());
        println!("{}", "(dry-run — no files written)".dimmed());
        return Ok(());
    }

    // ── Guard output file ─────────────────────────────────────────────────────
    if output.exists() && !args.force {
        return Err(AppError::OutputExists {
            path: output.clone(),
        }
        .into());
    }

    // ── Optional backup ───────────────────────────────────────────────────────
    if args.backup
        && let Some(bak) = backup_if_exists(&output)?
    {
        eprintln!("{} '{}'", "Backup :".blue().bold(), bak.display());
        if gguf.path == output {
            gguf.path = bak;
        }
    }

    // ── Apply all params ──────────────────────────────────────────────────────
    let mut changed = 0usize;
    for (key, value) in &params {
        let exists = gguf.metadata.contains_key(key);
        gguf.metadata.insert(key.to_string(), value.clone());
        if exists {
            eprintln!("  {} {}", "Updated :".yellow().bold(), key);
        } else {
            eprintln!("  {} {}", "Added   :".green().bold(), key);
        }
        changed += 1;
    }

    // ── Write ─────────────────────────────────────────────────────────────────
    write_modified_gguf(
        &gguf.path,
        gguf.tensor_data_offset,
        &gguf.metadata,
        &gguf.tensor_infos,
        gguf.alignment,
        &output,
    )
    .with_context(|| format!("write output '{}'", output.display()))?;

    eprintln!(
        "{} '{}' — {} sampling parameter(s) written ({} bytes)",
        "Written:".green().bold(),
        output.display(),
        changed,
        fs::metadata(&output).map(|m| m.len()).unwrap_or(0),
    );

    Ok(())
}

// ── Resolve from HuggingFace README ──────────────────────────────────────────

fn resolve_from_repo(
    repo: &str,
    mode: Option<&str>,
) -> anyhow::Result<HashMap<&'static str, MetadataValue>> {
    let repo_id = hf::RepoId::parse(repo)
        .with_context(|| format!("invalid repo: '{repo}' — expected 'owner/repo'"))?;

    eprintln!("{} {}", "Fetching README from".cyan().bold(), repo);
    let client = reqwest::blocking::Client::builder()
        .user_agent("gguf-analyzer/1")
        .build()?;

    let readme = hf::fetch_readme(&client, &repo_id)?;
    let Some(text) = readme else {
        eprintln!(
            "{}",
            "No README found in repository — switching to interactive mode.".yellow()
        );
        return prompt_interactive();
    };

    let card = ModelCard::parse(&text);

    if card.sampling_sets.is_empty() {
        eprintln!(
            "{}",
            "No sampling parameters found in README — switching to interactive mode.".yellow()
        );
        return prompt_interactive();
    }

    // Pick the appropriate set (or fall through to custom/interactive)
    pick_set_menu(&card.sampling_sets, mode)
}

/// Present a clean three-category menu (Thinking / Non-Thinking / Custom) and
/// return the resolved parameter map.
///
/// If `--mode` was supplied on the CLI the menu is skipped and the matching set
/// is used directly.  "custom" as the mode value always goes straight to the
/// interactive prompt.
fn pick_set_menu(
    sets: &[SamplingSet],
    mode: Option<&str>,
) -> anyhow::Result<HashMap<&'static str, MetadataValue>> {
    // ── --mode shortcut ───────────────────────────────────────────────────────
    if let Some(label) = mode {
        if label.to_lowercase() == "custom" {
            return prompt_interactive();
        }
        let label_lower = label.to_lowercase();
        if let Some(s) = sets
            .iter()
            .find(|s| s.label.to_lowercase().contains(&label_lower))
        {
            eprintln!("{} \"{}\"", "Using sampling set:".cyan().bold(), s.label);
            return Ok(sampling_set_to_map(s));
        }
        let available: Vec<&str> = sets.iter().map(|s| s.label.as_str()).collect();
        anyhow::bail!(
            "no sampling set matching '--mode {label}'; available: {}",
            available.join(", ")
        );
    }

    // ── Classify each set ─────────────────────────────────────────────────────
    let thinking: Vec<&SamplingSet> = sets
        .iter()
        .filter(|s| {
            s.label.to_lowercase().contains("thinking") && !s.label.to_lowercase().contains("non")
        })
        .collect();
    let non_thinking: Vec<&SamplingSet> = sets
        .iter()
        .filter(|s| s.label.to_lowercase().contains("non"))
        .collect();

    // ── Build menu entries ────────────────────────────────────────────────────
    // Each entry: (display label, Option<&SamplingSet>)
    // None → custom / interactive
    let mut entries: Vec<(&str, Option<&SamplingSet>)> = Vec::new();

    let thinking_set = thinking.first().copied();
    let non_thinking_set = non_thinking.first().copied();

    if thinking_set.is_some() {
        entries.push((
            "Thinking mode    (focused reasoning, lower temperature)",
            thinking_set,
        ));
    }
    if non_thinking_set.is_some() {
        entries.push((
            "Non-thinking mode  (fast conversational replies)",
            non_thinking_set,
        ));
    }
    // Any sets that didn't match either category get their own numbered entries
    for s in sets.iter() {
        let lbl = s.label.to_lowercase();
        if !lbl.contains("thinking") && !lbl.contains("non") {
            entries.push((&s.label, Some(s)));
        }
    }
    entries.push(("Custom           (enter parameters manually)", None));

    // ── Print menu ────────────────────────────────────────────────────────────
    eprintln!();
    eprintln!(
        "{}",
        "┌─ Sampling preset ────────────────────────────────────────┐"
            .cyan()
            .bold()
    );
    for (i, (label, _)) in entries.iter().enumerate() {
        eprintln!(
            "{}  {}  {}",
            "│".cyan().bold(),
            format!("[{}]", i).bold(),
            label,
        );
    }
    eprintln!(
        "{}",
        "└──────────────────────────────────────────────────────────┘"
            .cyan()
            .bold()
    );
    eprint!("Choose (0–{}, default 0): ", entries.len() - 1);

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let idx: usize = line.trim().parse().unwrap_or(0);
    let idx = idx.min(entries.len() - 1);

    let (chosen_label, chosen_set) = entries[idx];

    match chosen_set {
        None => {
            eprintln!();
            prompt_interactive()
        }
        Some(s) => {
            eprintln!(
                "{} \"{}\"",
                "Using preset:".cyan().bold(),
                chosen_label.trim()
            );
            Ok(sampling_set_to_map(s))
        }
    }
}

/// Convert a [`SamplingSet`] into a map of GGUF [`MetadataValue`]s.
///
/// Fields that couldn't be parsed as the expected numeric type are skipped with
/// a warning.
fn sampling_set_to_map(set: &SamplingSet) -> HashMap<&'static str, MetadataValue> {
    let mut map = HashMap::new();

    macro_rules! insert_f32 {
        ($opt:expr, $key:expr) => {
            if let Some(s) = &$opt {
                match s.trim().parse::<f32>() {
                    Ok(v) => {
                        let _ = map.insert($key, MetadataValue::F32(v));
                    }
                    Err(_) => {
                        eprintln!(
                            "{} cannot parse '{}' as f32 for key '{}' — skipping",
                            "Warning:".yellow().bold(),
                            s,
                            $key
                        );
                    }
                }
            }
        };
    }

    macro_rules! insert_u32 {
        ($opt:expr, $key:expr) => {
            if let Some(s) = &$opt {
                match s.trim().parse::<u32>() {
                    Ok(v) => {
                        let _ = map.insert($key, MetadataValue::U32(v));
                    }
                    Err(_) => {
                        // top_k is sometimes a float in README text (e.g. "20.0")
                        match s.trim().parse::<f32>() {
                            Ok(v) => {
                                let _ = map.insert($key, MetadataValue::U32(v as u32));
                            }
                            Err(_) => {
                                eprintln!(
                                    "{} cannot parse '{}' as u32 for key '{}' — skipping",
                                    "Warning:".yellow().bold(),
                                    s,
                                    $key
                                );
                            }
                        }
                    }
                }
            }
        };
    }

    insert_f32!(set.temperature, KEY_TEMPERATURE);
    insert_f32!(set.top_p, KEY_TOP_P);
    insert_u32!(set.top_k, KEY_TOP_K);
    insert_f32!(set.min_p, KEY_MIN_P);
    insert_f32!(set.presence_penalty, KEY_PRESENCE_PENALTY);
    insert_f32!(set.repetition_penalty, KEY_REPETITION_PENALTY);

    map
}

// ── Interactive prompt ────────────────────────────────────────────────────────

/// Prompt the user for each sampling parameter one at a time.
/// Each parameter is shown with a friendly description.
/// Blank input skips that parameter.
fn prompt_interactive() -> anyhow::Result<HashMap<&'static str, MetadataValue>> {
    eprintln!(
        "{}",
        "┌─ Custom sampling parameters ─────────────────────────────────────────────┐"
            .cyan()
            .bold()
    );
    eprintln!(
        "{} Press {} to skip any field and keep it unchanged.                  {}",
        "│".cyan().bold(),
        "Enter".bold(),
        "│".cyan().bold()
    );
    eprintln!(
        "{}",
        "└───────────────────────────────────────────────────────────────────────────┘"
            .cyan()
            .bold()
    );
    eprintln!();

    // (key, display name, type hint, friendly description)
    let params: &[(&str, &str, &str, &str)] = &[
        (
            KEY_TEMPERATURE,
            "Temperature",
            "0.0 – 2.0, e.g. 0.7",
            "Controls randomness. Lower = more focused, higher = more creative.",
        ),
        (
            KEY_TOP_K,
            "Top-K",
            "integer, e.g. 40",
            "Limits choices to the K most likely tokens. Lower = safer, 0 = disabled.",
        ),
        (
            KEY_TOP_P,
            "Top-P (nucleus)",
            "0.0 – 1.0, e.g. 0.9",
            "Keeps only tokens whose cumulative probability reaches P. Lower = less variety.",
        ),
        (
            KEY_MIN_P,
            "Min-P",
            "0.0 – 1.0, e.g. 0.05",
            "Drops tokens with probability below P × (top token probability). Cuts junk.",
        ),
        (
            KEY_PRESENCE_PENALTY,
            "Presence penalty",
            "−2.0 – 2.0, e.g. 0.0",
            "Penalises tokens that have already appeared. Positive = avoid repetition.",
        ),
    ];

    let mut map = HashMap::new();

    for (key, name, hint, description) in params {
        eprintln!("  {} {}", "▸".cyan(), name.bold());
        eprintln!("    {}", description.dimmed());
        loop {
            eprint!("    Value [{}]: ", hint);

            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                eprintln!("    {} skipped", "—".dimmed());
                eprintln!();
                break;
            }

            if *key == KEY_TOP_K {
                match trimmed
                    .parse::<u32>()
                    .or_else(|_| trimmed.parse::<f32>().map(|f| f as u32))
                {
                    Ok(v) => {
                        eprintln!("    {} {}", "✓".green(), v.to_string().cyan());
                        eprintln!();
                        map.insert(*key, MetadataValue::U32(v));
                        break;
                    }
                    Err(_) => {
                        eprintln!(
                            "    {} expected a whole number, got '{}' — try again",
                            "✗".red(),
                            trimmed
                        );
                    }
                }
            } else {
                match trimmed.parse::<f32>() {
                    Ok(v) => {
                        eprintln!("    {} {}", "✓".green(), v.to_string().cyan());
                        eprintln!();
                        map.insert(*key, MetadataValue::F32(v));
                        break;
                    }
                    Err(_) => {
                        eprintln!(
                            "    {} expected a decimal number, got '{}' — try again",
                            "✗".red(),
                            trimmed
                        );
                    }
                }
            }
        }
    }

    Ok(map)
}

// ── Dry-run display ───────────────────────────────────────────────────────────

fn print_params_table(params: &HashMap<&'static str, MetadataValue>, gguf: &ParsedGguf) {
    for (key, new_val) in params {
        if let Some(old_val) = gguf.metadata.get(key) {
            let old_str = format!("{old_val:?}");
            let new_str = format!("{new_val:?}");
            println!("  {} {} → {}", key.bold(), old_str.dimmed(), new_str.cyan());
        } else {
            let new_str = format!("{new_val:?}");
            println!("  {} {} (new key)", key.bold(), new_str.cyan());
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_set(
        label: &str,
        temp: Option<&str>,
        top_p: Option<&str>,
        top_k: Option<&str>,
        min_p: Option<&str>,
        presence: Option<&str>,
        repetition: Option<&str>,
    ) -> SamplingSet {
        SamplingSet {
            label: label.to_string(),
            temperature: temp.map(str::to_string),
            top_p: top_p.map(str::to_string),
            top_k: top_k.map(str::to_string),
            min_p: min_p.map(str::to_string),
            presence_penalty: presence.map(str::to_string),
            repetition_penalty: repetition.map(str::to_string),
            max_tokens: None,
            extra: vec![],
        }
    }

    #[test]
    fn converts_f32_fields() {
        let set = make_set(
            "test",
            Some("0.7"),
            Some("0.9"),
            None,
            Some("0.05"),
            None,
            None,
        );
        let map = sampling_set_to_map(&set);
        assert!(
            matches!(map.get(KEY_TEMPERATURE), Some(MetadataValue::F32(v)) if (*v - 0.7).abs() < 1e-5)
        );
        assert!(
            matches!(map.get(KEY_TOP_P), Some(MetadataValue::F32(v)) if (*v - 0.9).abs() < 1e-5)
        );
        assert!(
            matches!(map.get(KEY_MIN_P), Some(MetadataValue::F32(v)) if (*v - 0.05).abs() < 1e-5)
        );
        assert!(!map.contains_key(KEY_TOP_K));
    }

    #[test]
    fn converts_top_k_as_u32() {
        let set = make_set("test", None, None, Some("40"), None, None, None);
        let map = sampling_set_to_map(&set);
        assert!(matches!(map.get(KEY_TOP_K), Some(MetadataValue::U32(40))));
    }

    #[test]
    fn converts_top_k_float_string() {
        // Some READMEs write "20.0" for top-k
        let set = make_set("test", None, None, Some("20.0"), None, None, None);
        let map = sampling_set_to_map(&set);
        assert!(matches!(map.get(KEY_TOP_K), Some(MetadataValue::U32(20))));
    }

    #[test]
    fn skips_unparseable_field() {
        let set = make_set("test", Some("hot"), None, None, None, None, None);
        let map = sampling_set_to_map(&set);
        assert!(!map.contains_key(KEY_TEMPERATURE));
    }

    #[test]
    fn pick_set_by_mode_non_thinking() {
        let sets = vec![
            make_set("Thinking Mode", Some("0.6"), None, None, None, None, None),
            make_set(
                "Non-Thinking Mode",
                Some("0.7"),
                None,
                None,
                None,
                None,
                None,
            ),
        ];
        let map = pick_set_menu(&sets, Some("non-thinking")).unwrap();
        assert!(
            matches!(map.get(KEY_TEMPERATURE), Some(MetadataValue::F32(v)) if (*v - 0.7).abs() < 1e-5)
        );
    }

    #[test]
    fn pick_set_by_mode_thinking() {
        let sets = vec![
            make_set("Thinking Mode", Some("0.6"), None, None, None, None, None),
            make_set(
                "Non-Thinking Mode",
                Some("0.7"),
                None,
                None,
                None,
                None,
                None,
            ),
        ];
        let map = pick_set_menu(&sets, Some("thinking")).unwrap();
        assert!(
            matches!(map.get(KEY_TEMPERATURE), Some(MetadataValue::F32(v)) if (*v - 0.6).abs() < 1e-5)
        );
    }

    #[test]
    fn pick_set_missing_mode_errors() {
        let sets = vec![
            make_set("Alpha", None, None, None, None, None, None),
            make_set("Beta", None, None, None, None, None, None),
        ];
        assert!(pick_set_menu(&sets, Some("gamma")).is_err());
    }

    #[test]
    fn converts_repetition_penalty() {
        let set = make_set("test", None, None, None, None, None, Some("1.1"));
        let map = sampling_set_to_map(&set);
        assert!(
            matches!(map.get(KEY_REPETITION_PENALTY), Some(MetadataValue::F32(v)) if (*v - 1.1).abs() < 1e-4)
        );
    }
}

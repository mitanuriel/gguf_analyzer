//! `model-card` subcommand — fetch and parse a HuggingFace model README.
//!
//! Extracts:
//!  - YAML front-matter (license, tags, pipeline_tag, base model)
//!  - Model overview (parameter count, layers, context length, architecture)
//!  - Recommended sampling parameters (temperature, top-p, top-k, min-p,
//!    presence penalty, etc.)
//!
//! Prints a set of coloured tables, or JSON with `--json`.

use anyhow::Context as _;
use colored::Colorize as _;
use serde::Serialize;
use tabled::{Table, Tabled, settings::Style};

use crate::{
    cli::ModelCardArgs,
    hf::{self, RepoId},
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(args: &ModelCardArgs) -> anyhow::Result<()> {
    let client = hf::client()?;
    let repo = RepoId::parse(&args.repo)
        .with_context(|| format!("cannot parse repo from '{}'", args.repo))?;

    eprintln!(
        "{} {} …",
        "Fetching model card for".cyan().bold(),
        repo.id().bold()
    );

    let readme = hf::fetch_readme(&client, &repo)?
        .ok_or_else(|| anyhow::anyhow!("no README.md found in repo '{}'", repo.id()))?;

    let card = ModelCard::parse(&readme);

    if args.json {
        let json = serde_json::to_string_pretty(&card).context("serialise to JSON")?;
        println!("{json}");
    } else {
        print_card(&card, &repo);
    }

    Ok(())
}

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize)]
pub struct ModelCard {
    // Front-matter fields
    pub license: Option<String>,
    pub pipeline_tag: Option<String>,
    pub tags: Vec<String>,
    pub base_model: Option<String>,

    // Overview
    pub model_name: Option<String>,
    pub architecture: Option<String>,
    pub parameters: Option<String>,
    pub parameters_non_embedding: Option<String>,
    pub num_layers: Option<String>,
    pub attention_heads: Option<String>,
    pub context_length: Option<String>,
    pub quantization: Option<String>,

    // Sampling parameter sets
    pub sampling_sets: Vec<SamplingSet>,
}

/// A named group of sampling parameters (e.g. "thinking mode" vs "non-thinking
/// mode").
#[derive(Debug, Default, Serialize)]
pub struct SamplingSet {
    pub label: String,
    pub temperature: Option<String>,
    pub top_p: Option<String>,
    pub top_k: Option<String>,
    pub min_p: Option<String>,
    pub presence_penalty: Option<String>,
    pub repetition_penalty: Option<String>,
    pub max_tokens: Option<String>,
    pub extra: Vec<(String, String)>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

impl ModelCard {
    pub fn parse(text: &str) -> Self {
        let mut card = ModelCard::default();

        // ── YAML front-matter (between leading --- ... ---) ───────────────────
        if let Some(fm) = extract_frontmatter(text) {
            parse_frontmatter(fm, &mut card);
        }

        // ── Body ──────────────────────────────────────────────────────────────
        let body = strip_frontmatter(text);

        // Model name from first H1 or H2.
        if card.model_name.is_none() {
            card.model_name = extract_heading(body);
        }

        // Overview block.
        parse_overview(body, &mut card);

        // Sampling parameters.
        card.sampling_sets = parse_sampling(body);

        card
    }
}

// ── Front-matter helpers ──────────────────────────────────────────────────────

fn extract_frontmatter(text: &str) -> Option<&str> {
    let text = text.trim_start();
    if !text.starts_with("---") {
        return None;
    }
    let rest = &text[3..];
    // Skip optional newline right after opening ---
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn strip_frontmatter(text: &str) -> &str {
    let text = text.trim_start();
    if !text.starts_with("---") {
        return text;
    }
    let rest = &text[3..];
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    if let Some(end) = rest.find("\n---") {
        let after = &rest[end + 4..]; // skip "\n---"
        after.strip_prefix('\n').unwrap_or(after)
    } else {
        text
    }
}

fn parse_frontmatter(fm: &str, card: &mut ModelCard) {
    for line in fm.lines() {
        let line = line.trim();
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
            match key.as_str() {
                "license" => card.license = Some(val),
                "pipeline_tag" => card.pipeline_tag = Some(val),
                "base_model" => card.base_model = Some(val),
                _ => {}
            }
        } else if line.starts_with("- ") {
            // List items under `tags:`
            let tag = line.trim_start_matches("- ").trim().to_string();
            if !tag.is_empty() {
                card.tags.push(tag);
            }
        }
    }
}

// ── Overview helpers ──────────────────────────────────────────────────────────

fn extract_heading(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# ") {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn parse_overview(body: &str, card: &mut ModelCard) {
    // We look for bullet-point style "Label: value" lines (common in HF model
    // cards) as well as inline key: value patterns anywhere in the text.
    for line in body.lines() {
        let l = line.trim().trim_start_matches(['•', '-', '*', '+']).trim();
        // Strip markdown bold markers.
        let l = l.replace("**", "");
        let l = l.trim().to_string();

        let lower = l.to_lowercase();

        macro_rules! try_assign {
            ($field:expr, $($needle:expr),+) => {
                if $field.is_none() {
                    $(if lower.contains($needle) {
                        $field = extract_value_after_colon(&l);
                    })+
                }
            };
        }

        try_assign!(card.architecture, "architecture", "arch:");
        try_assign!(
            card.parameters,
            "number of parameters:",
            "num parameters",
            "total params"
        );
        try_assign!(
            card.parameters_non_embedding,
            "non-embedding",
            "non_embedding"
        );
        try_assign!(
            card.num_layers,
            "number of layers",
            "num_layers",
            "num layers"
        );
        try_assign!(
            card.attention_heads,
            "attention heads",
            "num_heads",
            "num heads"
        );
        try_assign!(
            card.context_length,
            "context length",
            "context_length",
            "max position"
        );
        try_assign!(card.quantization, "quantization:", "quantisation:");
    }
}

fn extract_value_after_colon(line: &str) -> Option<String> {
    line.split_once(':')
        .map(|(_, v)| v.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ── Sampling parameter parser ─────────────────────────────────────────────────
//
// Strategy:
//   1. Find sections that mention "sampling" or "best practice".
//   2. Within each section, look for bullet points / code block lines that
//      contain parameter names with `=` or `:` followed by a number.
//   3. Group by context line (e.g. "thinking mode", "non-thinking mode").

fn parse_sampling(body: &str) -> Vec<SamplingSet> {
    let mut sets: Vec<SamplingSet> = Vec::new();

    // Split body into lines for scanning.
    let lines: Vec<&str> = body.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let lower = lines[i].to_lowercase();

        // Detect a context label line (e.g. "For thinking mode (...)")
        let is_context_line = lower.contains("thinking mode")
            || lower.contains("non-thinking")
            || lower.contains("sampling param")
            || lower.contains("best practice");

        if is_context_line {
            // Collect the next ~10 lines that look like they contain params.
            let label = clean_label(lines[i]);
            let mut set = SamplingSet {
                label,
                ..Default::default()
            };

            let end = (i + 15).min(lines.len());
            for line in &lines[i..end] {
                extract_params_from_line(line, &mut set);
            }

            // Only keep sets that have at least one param.
            if set.has_any() {
                sets.push(set);
            }
        }

        // Also parse standalone command-line invocations (llama.cpp style).
        // e.g.: --temp 0.6 --top-k 20 --top-p 0.95 --min-p 0 --presence-penalty 1.5
        if lines[i].contains("--temp") || lines[i].contains("--temperature") {
            let label = find_preceding_label(&lines, i);
            let mut set = SamplingSet {
                label,
                ..Default::default()
            };
            extract_cli_flags(lines[i], &mut set);
            if set.has_any() {
                sets.push(set);
            }
        }

        i += 1;
    }

    // Deduplicate very similar sets (same label + same temperature).
    sets.dedup_by(|a, b| a.label == b.label && a.temperature == b.temperature);

    sets
}

fn extract_params_from_line(line: &str, set: &mut SamplingSet) {
    let line_lower = line.to_lowercase();

    // Match patterns like `Temperature=0.6`, `temp=0.6`, `TopP=0.95`, etc.
    let assignments: Vec<(&str, &str)> = vec![
        ("temperature", "temperature"),
        ("temp", "temperature"),
        ("top_p", "top_p"),
        ("topp", "top_p"),
        ("top-p", "top_p"),
        ("top_k", "top_k"),
        ("topk", "top_k"),
        ("top-k", "top_k"),
        ("min_p", "min_p"),
        ("minp", "min_p"),
        ("min-p", "min_p"),
        ("presence_penalty", "presence_penalty"),
        ("presencepenalty", "presence_penalty"),
        ("presence-penalty", "presence_penalty"),
        ("repetition_penalty", "repetition_penalty"),
        ("repetitionpenalty", "repetition_penalty"),
        ("max_tokens", "max_tokens"),
        ("max_new_tokens", "max_tokens"),
        ("n_ctx", "max_tokens"),
    ];

    for (needle, field) in &assignments {
        if line_lower.contains(needle)
            && let Some(val) = extract_numeric_value(line, needle)
        {
            match *field {
                "temperature" => _ = set.temperature.get_or_insert(val),
                "top_p" => _ = set.top_p.get_or_insert(val),
                "top_k" => _ = set.top_k.get_or_insert(val),
                "min_p" => _ = set.min_p.get_or_insert(val),
                "presence_penalty" => _ = set.presence_penalty.get_or_insert(val),
                "repetition_penalty" => _ = set.repetition_penalty.get_or_insert(val),
                "max_tokens" => _ = set.max_tokens.get_or_insert(val),
                _ => {}
            }
        }
    }
}

/// Extract a numeric (or boolean) value appearing after `key` in any of the
/// forms: `key=value`, `key: value`, `key value` (flag-style).
fn extract_numeric_value(line: &str, key: &str) -> Option<String> {
    let lower = line.to_lowercase();
    let key_lower = key.to_lowercase();

    let pos = lower.find(&key_lower)?;
    let after = &line[pos + key.len()..].trim_start_matches(['=', ':', ' ', '\t']);

    // Grab leading numeric / boolean token.
    let token: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();

    if token.is_empty() { None } else { Some(token) }
}

/// Parse llama.cpp-style CLI flags: `--temp 0.6 --top-k 20 …`
fn extract_cli_flags(line: &str, set: &mut SamplingSet) {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut j = 0;
    while j < tokens.len() {
        let tok = tokens[j];
        if let Some(val) = tokens.get(j + 1) {
            let val = val.trim_end_matches([',', ';', '\'', '"']);
            match tok {
                "--temp" | "--temperature" => set.temperature = Some(val.to_string()),
                "--top-p" => set.top_p = Some(val.to_string()),
                "--top-k" => set.top_k = Some(val.to_string()),
                "--min-p" => set.min_p = Some(val.to_string()),
                "--presence-penalty" => set.presence_penalty = Some(val.to_string()),
                "--repeat-penalty" | "--repetition-penalty" => {
                    set.repetition_penalty = Some(val.to_string())
                }
                "-n" | "--n-predict" => set.max_tokens = Some(val.to_string()),
                _ => {}
            }
            j += 2;
        } else {
            j += 1;
        }
    }
}

fn clean_label(line: &str) -> String {
    // Strip markdown formatting, bullets, and truncate.
    let l = line
        .trim()
        .trim_start_matches(['#', '-', '*', '>', ' '])
        .replace(['*', '_', '`'], "")
        .trim()
        .to_string();
    if l.len() > 80 {
        format!("{}…", &l[..79])
    } else {
        l
    }
}

fn find_preceding_label(lines: &[&str], i: usize) -> String {
    for j in (0..i).rev().take(5) {
        let trimmed = lines[j].trim();
        if !trimmed.is_empty() && !trimmed.starts_with("```") {
            return clean_label(trimmed);
        }
    }
    "llama.cpp invocation".to_string()
}

impl SamplingSet {
    fn has_any(&self) -> bool {
        self.temperature.is_some()
            || self.top_p.is_some()
            || self.top_k.is_some()
            || self.min_p.is_some()
            || self.presence_penalty.is_some()
            || self.repetition_penalty.is_some()
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

fn print_card(card: &ModelCard, repo: &RepoId) {
    println!();
    println!(
        "{}",
        format!("  Model Card: {}  ", repo.id()).bold().on_cyan()
    );
    println!();

    // ── Overview ─────────────────────────────────────────────────────────────
    println!(
        "{}",
        "═══ Overview ════════════════════════════════════"
            .cyan()
            .bold()
    );

    #[derive(Tabled)]
    struct KvRow {
        #[tabled(rename = "Field")]
        field: String,
        #[tabled(rename = "Value")]
        value: String,
    }

    macro_rules! kv_row {
        ($rows:ident, $label:expr, $opt:expr) => {
            if let Some(ref v) = $opt {
                $rows.push(KvRow {
                    field: $label.to_string(),
                    value: v.clone(),
                });
            }
        };
    }

    let mut overview_rows: Vec<KvRow> = Vec::new();
    kv_row!(overview_rows, "Model name", card.model_name);
    kv_row!(overview_rows, "Architecture", card.architecture);
    kv_row!(overview_rows, "Parameters", card.parameters);
    kv_row!(
        overview_rows,
        "Parameters (non-embed)",
        card.parameters_non_embedding
    );
    kv_row!(overview_rows, "Layers", card.num_layers);
    kv_row!(overview_rows, "Attention heads", card.attention_heads);
    kv_row!(overview_rows, "Context length", card.context_length);
    kv_row!(overview_rows, "Quantization", card.quantization);
    kv_row!(overview_rows, "License", card.license);
    kv_row!(overview_rows, "Pipeline", card.pipeline_tag);
    kv_row!(overview_rows, "Base model", card.base_model);
    if !card.tags.is_empty() {
        overview_rows.push(KvRow {
            field: "Tags".to_string(),
            value: card.tags.join(", "),
        });
    }

    if overview_rows.is_empty() {
        println!("  (no overview fields found)");
    } else {
        let mut table = Table::new(overview_rows);
        table.with(Style::rounded());
        println!("{table}");
    }

    // ── Sampling parameters ───────────────────────────────────────────────────
    if card.sampling_sets.is_empty() {
        println!(
            "\n{}",
            "No sampling parameters found in the model card.".dimmed()
        );
        return;
    }

    println!();
    println!(
        "{}",
        "═══ Recommended Sampling Parameters ════════════"
            .cyan()
            .bold()
    );

    for set in &card.sampling_sets {
        println!("\n  {}", set.label.bold().yellow());

        #[derive(Tabled)]
        struct ParamRow {
            #[tabled(rename = "Parameter")]
            param: String,
            #[tabled(rename = "Value")]
            value: String,
        }

        let mut rows: Vec<ParamRow> = Vec::new();

        macro_rules! param_row {
            ($label:expr, $opt:expr) => {
                if let Some(ref v) = $opt {
                    rows.push(ParamRow {
                        param: $label.to_string(),
                        value: v.clone(),
                    });
                }
            };
        }

        param_row!("Temperature", set.temperature);
        param_row!("Top-P", set.top_p);
        param_row!("Top-K", set.top_k);
        param_row!("Min-P", set.min_p);
        param_row!("Presence Penalty", set.presence_penalty);
        param_row!("Repetition Penalty", set.repetition_penalty);
        param_row!("Max Tokens", set.max_tokens);
        for (k, v) in &set.extra {
            rows.push(ParamRow {
                param: k.clone(),
                value: v.clone(),
            });
        }

        if rows.is_empty() {
            println!("    (no params extracted)");
        } else {
            let mut table = Table::new(rows);
            table.with(Style::rounded());
            // Indent the table slightly.
            for line in table.to_string().lines() {
                println!("  {line}");
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const QWEN_SNIPPET: &str = r#"---
license: apache-2.0
pipeline_tag: text-generation
tags:
- conversational
base_model: Qwen/Qwen3-0.6B
---
# Qwen3-0.6B-GGUF

## Model Overview

- Number of Parameters: 0.6B
- Number of Layers: 28
- Number of Attention Heads (GQA): 16 for Q and 8 for KV
- Context Length: 32768
- Quantization: q8_0

## Best Practices

- For thinking mode (`enable_thinking=True`): Temperature=0.6, TopP=0.95, TopK=20, MinP=0, and PresencePenalty=1.5
- For non-thinking mode (`enable_thinking=False`): Temperature=0.7, TopP=0.8, TopK=20, MinP=0, and PresencePenalty=1.5
"#;

    #[test]
    fn parses_frontmatter_license() {
        let card = ModelCard::parse(QWEN_SNIPPET);
        assert_eq!(card.license.as_deref(), Some("apache-2.0"));
    }

    #[test]
    fn parses_frontmatter_pipeline_tag() {
        let card = ModelCard::parse(QWEN_SNIPPET);
        assert_eq!(card.pipeline_tag.as_deref(), Some("text-generation"));
    }

    #[test]
    fn parses_frontmatter_tags() {
        let card = ModelCard::parse(QWEN_SNIPPET);
        assert!(card.tags.contains(&"conversational".to_string()));
    }

    #[test]
    fn parses_model_name() {
        let card = ModelCard::parse(QWEN_SNIPPET);
        assert_eq!(card.model_name.as_deref(), Some("Qwen3-0.6B-GGUF"));
    }

    #[test]
    fn parses_context_length() {
        let card = ModelCard::parse(QWEN_SNIPPET);
        assert!(card.context_length.is_some());
    }

    #[test]
    fn parses_sampling_parameters() {
        let card = ModelCard::parse(QWEN_SNIPPET);
        assert!(
            !card.sampling_sets.is_empty(),
            "should find at least one sampling set"
        );
        let thinking = card
            .sampling_sets
            .iter()
            .find(|s| s.label.to_lowercase().contains("thinking"));
        assert!(thinking.is_some(), "should find a thinking-mode set");
        let t = thinking.unwrap();
        assert_eq!(t.temperature.as_deref(), Some("0.6"));
        assert_eq!(t.top_p.as_deref(), Some("0.95"));
        assert_eq!(t.top_k.as_deref(), Some("20"));
    }

    #[test]
    fn extract_frontmatter_returns_none_without_delimiters() {
        assert!(extract_frontmatter("# Just a heading\nNo frontmatter here").is_none());
    }
}

//! `info` subcommand — print a high-level summary of a GGUF file.

use anyhow::Context as _;
use std::path::Path;

use crate::{
    cli::InfoArgs,
    display::{format_bytes, term_width},
    gguf::ParsedGguf,
};
use tabled::{builder::Builder, settings::Style, Table};

/// Run the `info` subcommand.
pub fn run(args: &InfoArgs) -> anyhow::Result<()> {
    let gguf = ParsedGguf::open(&args.file)
        .with_context(|| format!("failed to open '{}'", args.file.display()))?;

    let width = term_width();
    let table = info_table(&gguf, width);
    println!("{}", table);
    Ok(())
}

fn info_table(gguf: &ParsedGguf, _width: usize) -> Table {
    let mut builder = Builder::new();
    builder.push_record(["Field", "Value"]);
    builder.push_record(["File", &gguf.path.display().to_string()]);
    builder.push_record(["File size", &format_bytes(gguf.file_size)]);
    builder.push_record(["GGUF version", &gguf.version.to_string()]);
    builder.push_record(["Tensor count", &gguf.tensor_count.to_string()]);
    builder.push_record(["Metadata entries", &gguf.metadata.len().to_string()]);
    builder.push_record(["Alignment", &format!("{} bytes", gguf.alignment)]);
    builder.push_record([
        "Tensor data offset",
        &format!("{:#010x}  ({})", gguf.tensor_data_offset, format_bytes(gguf.tensor_data_offset)),
    ]);

    let mut table = builder.build();
    table.with(Style::rounded());
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn info_on_missing_file_returns_error() {
        let args = InfoArgs { file: PathBuf::from("/no/such/file.gguf") };
        assert!(run(&args).is_err());
    }
}

//! Display helpers: value formatting, byte sizes, and terminal-aware tables.
//!
//! All table-building functions use [`tabled`] with [`terminal_size`] so output
//! never overflows the current terminal width (falls back to 80 columns).

use colored::Colorize as _;
use gguf_rs_lib::{
    format::metadata::{MetadataArray, MetadataValue},
    tensor::info::TensorInfo,
};
use tabled::{
    builder::Builder,
    settings::{
        object::Columns,
        Modify, Style, Width,
    },
    Table,
};
use terminal_size::{terminal_size, Width as TWidth};

// ── Terminal width ────────────────────────────────────────────────────────────

/// Returns the current terminal width, or 80 if it cannot be determined.
pub fn term_width() -> usize {
    terminal_size()
        .map(|(TWidth(w), _)| w as usize)
        .unwrap_or(80)
}

// ── Byte formatting ───────────────────────────────────────────────────────────

/// Format a byte count as a human-readable string (IEC units).
///
/// ```
/// use gguf_analyzer::display::format_bytes;
/// assert_eq!(format_bytes(0),               "0 B");
/// assert_eq!(format_bytes(1023),            "1023 B");
/// assert_eq!(format_bytes(1024),            "1.00 KiB");
/// assert_eq!(format_bytes(1024 * 1024),     "1.00 MiB");
/// assert_eq!(format_bytes(1024u64.pow(3)),  "1.00 GiB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    const TIB: u64 = GIB * 1024;

    if bytes < KIB {
        format!("{} B", bytes)
    } else if bytes < MIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else if bytes < GIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes < TIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    }
}

// ── MetadataValue formatting ──────────────────────────────────────────────────

/// Format a [`MetadataValue`] as a compact, human-readable string.
///
/// Array values are truncated to `array_limit` elements; if the array is
/// longer a `[… N more]` suffix is appended.
pub fn format_value(val: &MetadataValue, array_limit: usize) -> String {
    match val {
        MetadataValue::U8(v)     => v.to_string(),
        MetadataValue::I8(v)     => v.to_string(),
        MetadataValue::U16(v)    => v.to_string(),
        MetadataValue::I16(v)    => v.to_string(),
        MetadataValue::U32(v)    => v.to_string(),
        MetadataValue::I32(v)    => v.to_string(),
        MetadataValue::F32(v)    => format!("{}", v),
        MetadataValue::U64(v)    => v.to_string(),
        MetadataValue::I64(v)    => v.to_string(),
        MetadataValue::F64(v)    => format!("{}", v),
        MetadataValue::Bool(v)   => v.to_string(),
        MetadataValue::String(v) => v.clone(),
        MetadataValue::Array(arr)  => format_array(arr, array_limit),
    }
}

/// Format a [`MetadataValue`] type tag as a short string (e.g. `"u32"`, `"string"`).
pub fn format_type(val: &MetadataValue) -> &'static str {
    match val {
        MetadataValue::U8(_)     => "u8",
        MetadataValue::I8(_)     => "i8",
        MetadataValue::U16(_)    => "u16",
        MetadataValue::I16(_)    => "i16",
        MetadataValue::U32(_)    => "u32",
        MetadataValue::I32(_)    => "i32",
        MetadataValue::F32(_)    => "f32",
        MetadataValue::U64(_)    => "u64",
        MetadataValue::I64(_)    => "i64",
        MetadataValue::F64(_)    => "f64",
        MetadataValue::Bool(_)   => "bool",
        MetadataValue::String(_) => "string",
        MetadataValue::Array(_)  => "array",    }
}

fn format_array(arr: &MetadataArray, limit: usize) -> String {
    let total = arr.len();
    let shown = total.min(limit);
    let parts: Vec<String> = (0..shown)
        .filter_map(|i| arr.get(i))
        .map(|v| format_value(v, limit))
        .collect();

    if total > shown {
        format!("[{}, … {} more]", parts.join(", "), total - shown)
    } else {
        format!("[{}]", parts.join(", "))
    }
}

// ── Tensor shape formatting ───────────────────────────────────────────────────

/// Format a tensor shape as `[d0 × d1 × …]`.
pub fn format_shape(dims: &[u64]) -> String {
    if dims.is_empty() {
        return "[]".to_string();
    }
    let parts: Vec<String> = dims.iter().map(|d| d.to_string()).collect();
    format!("[{}]", parts.join(" × "))
}

// ── Table builders ────────────────────────────────────────────────────────────

/// Build a [`Table`] from key-value rows, constrained to `width` columns.
///
/// The header row is `["Key", "Value"]`.
pub fn kv_table(rows: &[(&str, &str)], width: usize) -> Table {
    let mut builder = Builder::new();
    builder.push_record([
        "Key".bold().cyan().to_string(),
        "Value".bold().cyan().to_string(),
    ]);
    for (k, v) in rows {
        builder.push_record([k.bold().white().to_string(), (*v).to_string()]);
    }
    let mut table = builder.build();
    table.with(Style::rounded());
    table.with(Width::wrap(width));
    table
}

/// Build a three-column `["Key", "Type", "Value"]` table for metadata display.
///
/// Key and Type columns keep their natural widths; the Value column wraps to
/// fill the remaining terminal width.
pub fn meta_table(rows: &[(&str, &str, &str)], width: usize) -> Table {
    let mut builder = Builder::new();
    builder.push_record([
        "Key".bold().cyan().to_string(),
        "Type".bold().cyan().to_string(),
        "Value".bold().cyan().to_string(),
    ]);
    for (k, t, v) in rows {
        builder.push_record([
            k.bold().white().to_string(),
            t.dimmed().to_string(),
            (*v).to_string(),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::rounded());
    // Let the whole table use at most `width` chars, wrapping the Value column.
    // Key (col 0) and Type (col 1) get a MinWidth so they are never squashed.
    let key_w   = rows.iter().map(|(k,_,_)| k.len()).max().unwrap_or(3).max(3);
    let type_w  = 8_usize; // longest type name is "string" (6) + borders
    let borders = 4 * 3 + 2; // 4 col-borders × ~3 chars + outer
    let value_w = width.saturating_sub(key_w + type_w + borders).max(20);
    table.with(Modify::new(Columns::new(..1)).with(Width::increase(key_w)));
    table.with(Modify::new(Columns::new(1..2)).with(Width::increase(type_w)));
    table.with(Modify::new(Columns::new(2..)).with(Width::wrap(value_w)));
    table
}

/// Build the tensor info table.
///
/// Columns: `Name | Shape | Type | Offset | Size`.
pub fn tensor_table(infos: &[TensorInfo], width: usize) -> Table {
    let mut builder = Builder::new();
    builder.push_record([
        "Name".bold().cyan().to_string(),
        "Shape".bold().cyan().to_string(),
        "Type".bold().cyan().to_string(),
        "Offset".bold().cyan().to_string(),
        "Size".bold().cyan().to_string(),
    ]);
    for ti in infos {
        let shape = format_shape(ti.shape.dims());
        let type_name = ti.tensor_type.name().to_string();
        let offset = format!("{:#010x}", ti.data_offset);
        let size = format_bytes(ti.expected_data_size());
        builder.push_record([
            ti.name.bold().white().to_string(),
            shape,
            type_name.yellow().to_string(),
            offset.dimmed().to_string(),
            size.green().to_string(),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::rounded());
    table.with(Width::wrap(width));
    table
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── format_bytes ─────────────────────────────────────────────────────────

    #[test]
    fn bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn bytes_under_kib() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn bytes_kib() {
        assert_eq!(format_bytes(1024), "1.00 KiB");
        assert_eq!(format_bytes(2048), "2.00 KiB");
    }

    #[test]
    fn bytes_mib() {
        assert_eq!(format_bytes(1024 * 1024), "1.00 MiB");
    }

    #[test]
    fn bytes_gib() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GiB");
    }

    // ── format_value ─────────────────────────────────────────────────────────

    #[test]
    fn format_scalar_values() {
        assert_eq!(format_value(&MetadataValue::U32(42), 8), "42");
        assert_eq!(format_value(&MetadataValue::Bool(true), 8), "true");
        assert_eq!(
            format_value(&MetadataValue::String("llama".to_string()), 8),
            "llama"
        );
    }

    #[test]
    fn format_type_names() {
        assert_eq!(format_type(&MetadataValue::U8(0)), "u8");
        assert_eq!(format_type(&MetadataValue::F32(0.0)), "f32");
        assert_eq!(format_type(&MetadataValue::String(String::new())), "string");
        assert_eq!(
            format_type(&MetadataValue::Array(Box::new(
                MetadataArray::new(
                    gguf_rs_lib::format::types::GGUFValueType::U32,
                    vec![]
                )
                .unwrap()
            ))),
            "array"
        );
    }

    // ── format_shape ─────────────────────────────────────────────────────────

    #[test]
    fn shape_empty() {
        assert_eq!(format_shape(&[]), "[]");
    }

    #[test]
    fn shape_vector() {
        assert_eq!(format_shape(&[512]), "[512]");
    }

    #[test]
    fn shape_matrix() {
        assert_eq!(format_shape(&[4096, 4096]), "[4096 × 4096]");
    }

    #[test]
    fn shape_3d() {
        assert_eq!(format_shape(&[2, 4, 8]), "[2 × 4 × 8]");
    }

    // ── term_width fallback ───────────────────────────────────────────────────

    #[test]
    fn term_width_is_positive() {
        // In a CI / test context there may be no TTY; the fallback is 80.
        assert!(term_width() >= 20);
    }
}

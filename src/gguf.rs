//! GGUF file access layer.
//!
//! [`ParsedGguf`] opens a GGUF file using **memory-mapping** (via
//! [`memmap2`] + `gguf-rs-lib`'s `MmapGGUFFile`) so that only the header
//! region is paged in by the OS – the gigabyte-sized tensor blob is never
//! copied into the process heap.
//!
//! For *write* operations (`set`, `remove`) the module provides
//! [`write_modified_gguf`], which:
//!
//! 1. Re-serialises the new header + modified metadata.
//! 2. Calculates the GGUF-spec alignment padding between the header and the
//!    first tensor (`general.alignment`, defaulting to 32 bytes).
//! 3. Streams the unchanged tensor data from the mmap'd source into the new
//!    file.

use std::{
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use gguf_rs_lib::{
    format::metadata::Metadata,
    mmap::MmapGGUFFile,
    reader::file_reader::GGUFFileReader,
    tensor::info::TensorInfo,
    GGUFError,
};

use crate::error::AppError;

// ── Default GGUF alignment (spec §File Structure) ─────────────────────────────
pub const DEFAULT_ALIGNMENT: u64 = 32;

// ── Public data structure ─────────────────────────────────────────────────────

/// A fully-parsed GGUF file, loaded via memory-map.
///
/// Accessing [`metadata`](ParsedGguf::metadata) and
/// [`tensor_infos`](ParsedGguf::tensor_infos) is always cheap – the
/// heavy tensor bytes stay on disk.
pub struct ParsedGguf {
    /// Path the file was opened from (used in error messages).
    pub path: PathBuf,
    /// GGUF format version (currently 3).
    pub version: u32,
    /// Number of tensors declared in the header.
    pub tensor_count: u64,
    /// All metadata key-value pairs.
    pub metadata: Metadata,
    /// Tensor info entries (name, shape, type, offset).
    pub tensor_infos: Vec<TensorInfo>,
    /// Byte offset inside the file where tensor data begins.
    pub tensor_data_offset: u64,
    /// Raw alignment value from `general.alignment` (or the spec default).
    pub alignment: u64,
    /// Total file size in bytes.
    pub file_size: u64,
    /// The live memory map (keeps the pages alive).
    _mmap: MmapGGUFFile,
}

// ── Constructor ───────────────────────────────────────────────────────────────

impl ParsedGguf {
    /// Open a GGUF file using a memory map.
    ///
    /// # Errors
    /// Returns [`AppError::Io`] if the file cannot be opened, or
    /// [`AppError::GgufLib`] / [`AppError::GgufParse`] if it is corrupt.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let file_size = fs::metadata(&path)
            .map_err(|e| AppError::io(&path, e))
            .with_context(|| format!("stat('{}')", path.display()))?
            .len();

        // Memory-map the file
        let mmap = MmapGGUFFile::mmap(&path)
            .map_err(|e| AppError::gguf_lib(&path, e))
            .with_context(|| format!("mmap('{}')", path.display()))?;

        // Open through a reader to parse header + metadata + tensor infos.
        // `open_gguf_file` uses std::fs::File internally; the mmap above is
        // separate and only used to keep the mapping alive for write paths.
        let file = fs::File::open(&path)
            .map_err(|e| AppError::io(&path, e))
            .with_context(|| format!("open('{}')", path.display()))?;

        let reader = GGUFFileReader::new(io::BufReader::new(file))
            .map_err(|e| AppError::gguf_lib(&path, e))
            .with_context(|| format!("parse header of '{}'", path.display()))?;

        let header = reader.header();
        let version = header.version;
        let tensor_count = header.tensor_count;
        let tensor_data_offset = reader.tensor_data_offset();

        let metadata = reader.metadata().clone();
        let tensor_infos = reader.tensor_infos().to_vec();

        // Respect the model's declared alignment; fall back to spec default.
        let alignment = metadata
            .get_u64("general.alignment")
            .unwrap_or(DEFAULT_ALIGNMENT);

        Ok(Self {
            path,
            version,
            tensor_count,
            metadata,
            tensor_infos,
            tensor_data_offset,
            alignment,
            file_size,
            _mmap: mmap,
        })
    }
}

// ── Alignment helper ──────────────────────────────────────────────────────────

/// Round `offset` up to the next multiple of `alignment`.
///
/// When `offset` is already aligned this returns `offset` unchanged.
///
/// ```
/// use gguf_analyzer::gguf::align_offset;
/// assert_eq!(align_offset(0,  32), 0);
/// assert_eq!(align_offset(1,  32), 32);
/// assert_eq!(align_offset(32, 32), 32);
/// assert_eq!(align_offset(33, 32), 64);
/// ```
pub fn align_offset(offset: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return offset;
    }
    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + (alignment - remainder)
    }
}

// ── Write helper ──────────────────────────────────────────────────────────────

/// Write a new GGUF file to `dest` with a modified `metadata`.
///
/// The tensor data section is **byte-for-byte identical** to the source —
/// it is streamed directly from `src_path` starting at `src_tensor_offset`,
/// so the weights are never decoded or re-encoded.
///
/// Alignment padding (zero bytes) is inserted between the header and the
/// tensor data to satisfy the GGUF spec.
///
/// # Arguments
/// * `src_path`          – source GGUF file to stream tensor data from.
/// * `src_tensor_offset` – byte offset of the tensor data section in `src_path`.
/// * `metadata`          – the modified metadata to serialise.
/// * `tensor_infos`      – tensor info entries (unchanged from source).
/// * `alignment`         – byte alignment for the tensor data section.
/// * `dest`              – destination path to write to.
pub fn write_modified_gguf(
    src_path: &Path,
    src_tensor_offset: u64,
    metadata: &Metadata,
    tensor_infos: &[TensorInfo],
    alignment: u64,
    dest: &Path,
) -> anyhow::Result<()> {
    use gguf_rs_lib::format::header::GGUFHeader;

    // ── 1. Build new header ────────────────────────────────────────────────
    let new_header = GGUFHeader::new(tensor_infos.len() as u64, metadata.len() as u64);

    // ── 2. Serialise header + metadata + tensor infos into a buffer ────────
    let mut header_buf: Vec<u8> = Vec::new();
    new_header
        .write_to(&mut header_buf)
        .map_err(|e| AppError::gguf_lib(dest, e))
        .context("serialise header")?;

    metadata
        .write_to(&mut header_buf)
        .map_err(|e| AppError::gguf_lib(dest, e))
        .context("serialise metadata")?;

    // Serialise each tensor info
    for ti in tensor_infos {
        // gguf-rs-lib exposes write_to on TensorInfo via its own trait
        use gguf_rs_lib::format::header::TensorInfo as HeaderTensorInfo;
        let hti = HeaderTensorInfo {
            name: ti.name.clone(),
            dimensions: ti.shape.dims().to_vec(),
            tensor_type: ti.tensor_type,
            offset: ti.data_offset,
        };
        hti.write_to(&mut header_buf)
            .map_err(|e| AppError::gguf_lib(dest, e))
            .context("serialise tensor info")?;
    }

    // ── 3. Calculate alignment padding ────────────────────────────────────
    let header_end = header_buf.len() as u64;
    let aligned_end = align_offset(header_end, alignment);
    let padding_bytes = (aligned_end - header_end) as usize;

    // ── 4. Open source for tensor streaming ───────────────────────────────
    let mut src_file = fs::File::open(src_path)
        .map_err(|e| AppError::io(src_path, e))
        .context("open source for tensor copy")?;

    use std::io::{Read, Seek, SeekFrom};
    src_file
        .seek(SeekFrom::Start(src_tensor_offset))
        .map_err(|e| AppError::io(src_path, e))
        .context("seek to tensor data")?;

    // ── 5. Write everything to dest ────────────────────────────────────────
    let dest_file = fs::File::create(dest)
        .map_err(|e| AppError::io(dest, e))
        .context("create output file")?;

    let mut writer = BufWriter::new(dest_file);
    writer
        .write_all(&header_buf)
        .map_err(|e| AppError::io(dest, e))
        .context("write header")?;

    // Alignment padding (zero bytes)
    if padding_bytes > 0 {
        writer
            .write_all(&vec![0u8; padding_bytes])
            .map_err(|e| AppError::io(dest, e))
            .context("write alignment padding")?;
    }

    // Stream tensor data
    let mut buf = vec![0u8; 1024 * 1024]; // 1 MiB chunks
    loop {
        let n = src_file
            .read(&mut buf)
            .map_err(|e| AppError::io(src_path, e))
            .context("read tensor data")?;
        if n == 0 {
            break;
        }
        writer
            .write_all(&buf[..n])
            .map_err(|e| AppError::io(dest, e))
            .context("write tensor data")?;
    }

    writer
        .flush()
        .map_err(|e| AppError::io(dest, e))
        .context("flush output")?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── align_offset ─────────────────────────────────────────────────────────

    #[test]
    fn align_zero_offset_stays_zero() {
        assert_eq!(align_offset(0, 32), 0);
    }

    #[test]
    fn align_already_aligned_unchanged() {
        assert_eq!(align_offset(32, 32), 32);
        assert_eq!(align_offset(64, 32), 64);
        assert_eq!(align_offset(256, 32), 256);
    }

    #[test]
    fn align_one_over_rounds_up() {
        assert_eq!(align_offset(33, 32), 64);
        assert_eq!(align_offset(1, 32), 32);
    }

    #[test]
    fn align_spec_default_is_32() {
        // The GGUF spec says default alignment is 32
        assert_eq!(DEFAULT_ALIGNMENT, 32);
    }

    #[test]
    fn align_zero_alignment_returns_offset_unchanged() {
        // Guard against division-by-zero
        assert_eq!(align_offset(17, 0), 17);
    }

    #[test]
    fn align_with_alignment_8() {
        assert_eq!(align_offset(0, 8), 0);
        assert_eq!(align_offset(7, 8), 8);
        assert_eq!(align_offset(8, 8), 8);
        assert_eq!(align_offset(9, 8), 16);
    }

    #[test]
    fn padding_calculation_correct() {
        // header_end = 100, alignment = 32 → aligned = 128, padding = 28
        let header_end: u64 = 100;
        let alignment: u64 = 32;
        let aligned = align_offset(header_end, alignment);
        let padding = aligned - header_end;
        assert_eq!(padding, 28);
    }
}

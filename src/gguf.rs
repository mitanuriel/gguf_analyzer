//! GGUF file access layer.
//!
//! [`ParsedGguf`] opens a GGUF file using a **memory map** (via [`memmap2`])
//! so that the gigabyte-sized tensor data section is never copied into the
//! process heap.
//!
//! For *write* operations (`set`, `remove`) this module provides
//! [`write_modified_gguf`], which:
//!
//! 1. Re-serialises the new header + modified metadata.
//! 2. Inserts GGUF-spec alignment padding between the header and the first
//!    tensor (`general.alignment`, defaulting to 32 bytes).
//! 3. Streams the unchanged tensor data from the source file into the new file.

use std::{
    fs,
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use memmap2::Mmap;
use tracing::{debug, info, instrument};

use gguf_rs_lib::{
    format::{
        header::{GGUFHeader, TensorInfo as HeaderTensorInfo},
        metadata::Metadata,
    },
    reader::file_reader::GGUFFileReader,
    tensor::info::TensorInfo,
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
    /// Live memory map — keeps the mapping alive and available for tensor-data
    /// streaming in write operations.
    pub mmap: Mmap,
}

// ── Constructor ───────────────────────────────────────────────────────────────

impl ParsedGguf {
    /// Open a GGUF file using a memory map.
    ///
    /// # Errors
    /// Returns [`AppError::Io`] if the file cannot be opened, or
    /// [`AppError::GgufLib`] / [`AppError::GgufParse`] if it is corrupt.
    #[instrument(skip_all, fields(path = %path.as_ref().display()))]
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();

        let file_size = fs::metadata(&path)
            .map_err(|e| AppError::io(&path, e))
            .with_context(|| format!("stat '{}'", path.display()))?
            .len();

        // Open the file once for memory-mapping, once for the parser.
        let raw = fs::File::open(&path)
            .map_err(|e| AppError::io(&path, e))
            .with_context(|| format!("open '{}' for mmap", path.display()))?;

        // SAFETY: we never mutate the mapping and the file descriptor stays
        // open (via `raw`) until `mmap` is dropped with `ParsedGguf`.
        let mmap = unsafe { Mmap::map(&raw) }
            .map_err(|e| AppError::io(&path, e))
            .with_context(|| format!("mmap '{}'", path.display()))?;

        // Memory-map the file
        let file = fs::File::open(&path)
            .map_err(|e| AppError::io(&path, e))
            .with_context(|| format!("open '{}'", path.display()))?;

        let reader = GGUFFileReader::new(io::BufReader::new(file))
            .map_err(|e| AppError::gguf_lib(&path, e))
            .with_context(|| format!("parse '{}'", path.display()))?;

        let version = reader.header().version;
        let tensor_count = reader.header().tensor_count;
        let tensor_data_offset = reader.tensor_data_offset();

        let metadata = reader.metadata().clone();
        let tensor_infos = reader.tensor_infos().to_vec();

        // Respect the model's declared alignment; fall back to spec default.
        let alignment = metadata
            .get_u64("general.alignment")
            .unwrap_or(DEFAULT_ALIGNMENT);

        info!(
            version,
            tensor_count,
            metadata_entries = metadata.len(),
            alignment,
            tensor_data_offset,
            file_size,
            "parsed GGUF file"
        );

        Ok(Self {
            path,
            version,
            tensor_count,
            metadata,
            tensor_infos,
            tensor_data_offset,
            alignment,
            file_size,
            mmap,
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

// ── Backup helper ─────────────────────────────────────────────────────────────

/// If `path` exists, rename it to `<path>.bak` and return the backup path.
///
/// Any pre-existing `<path>.bak` is overwritten by `fs::rename`. Returns
/// `Ok(None)` if `path` did not exist (nothing to back up).
///
/// This is intentionally a *rename* (atomic on the same filesystem) rather
/// than a copy, so it's cheap even for large GGUF files.
pub fn backup_if_exists(path: &Path) -> anyhow::Result<Option<std::path::PathBuf>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let bak_path = std::path::PathBuf::from(bak);
    fs::rename(path, &bak_path).map_err(|e| AppError::io(path, e))?;
    Ok(Some(bak_path))
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
#[instrument(skip(metadata, tensor_infos), fields(dest = %dest.display()))]
pub fn write_modified_gguf(
    src_path: &Path,
    src_tensor_offset: u64,
    metadata: &Metadata,
    tensor_infos: &[TensorInfo],
    alignment: u64,
    dest: &Path,
) -> anyhow::Result<()> {
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
        let hti = HeaderTensorInfo {
            name: ti.name.clone(),
            n_dimensions: ti.shape.ndim() as u32,
            dimensions: ti.shape.dims().to_vec(),
            tensor_type: ti.tensor_type as u32,
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
    debug!(header_end, aligned_end, padding_bytes, "alignment calculated");

    // ── 4. Open source for tensor streaming ───────────────────────────────
    let mut src_file = fs::File::open(src_path)
        .map_err(|e| AppError::io(src_path, e))
        .context("open source for tensor copy")?;

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

    info!("write complete");
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

    // ── backup_if_exists ─────────────────────────────────────────────────────

    #[test]
    fn backup_returns_none_for_missing_path() {
        let p = std::path::PathBuf::from("/no/such/file/for/backup_test.gguf");
        let result = backup_if_exists(&p).expect("ok");
        assert!(result.is_none());
    }

    #[test]
    fn backup_renames_existing_file() {
        let dir = std::env::temp_dir().join(format!(
            "gguf_backup_test_{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let target = dir.join("data.gguf");
        let bak    = dir.join("data.gguf.bak");
        let _ = fs::remove_file(&target);
        let _ = fs::remove_file(&bak);

        fs::write(&target, b"original-bytes").unwrap();
        let returned = backup_if_exists(&target).expect("backup ok");

        assert_eq!(returned.as_deref(), Some(bak.as_path()));
        assert!(bak.exists(), ".bak should now exist");
        assert!(!target.exists(), "original path should have been moved");
        assert_eq!(fs::read(&bak).unwrap(), b"original-bytes");

        let _ = fs::remove_dir_all(&dir);
    }
}

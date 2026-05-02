//! Shared test helpers.
//!
//! [`minimal_gguf`] writes a small but fully valid GGUF v3 file to a
//! [`tempfile::NamedTempFile`] and returns both the temp-file handle (so the
//! file stays alive for the duration of the test) and the path to it.
//!
//! The fixture contains:
//! - 3 metadata keys: `general.architecture` (string "test-arch"),
//!   `general.name` (string "test-model"), `llm.context_length` (u32 = 512)
//! - 1 F32 tensor named "token_embd.weight" with shape [4, 8] and 128 zero bytes

use std::io::{BufWriter, Cursor};
use std::path::PathBuf;

use gguf_rs_lib::{
    format::metadata::{Metadata, MetadataValue},
    format::types::GGUFTensorType,
    tensor::{data::TensorData, info::TensorInfo, shape::TensorShape},
    writer::file_writer::GGUFFileWriter,
};
use tempfile::NamedTempFile;

/// Write a minimal valid GGUF file into a temporary file.
///
/// Returns `(NamedTempFile, PathBuf)` — keep the first value alive for the
/// duration of the test so the OS doesn't delete the file.
pub fn minimal_gguf() -> (NamedTempFile, PathBuf) {
    let mut metadata = Metadata::new();
    metadata.insert(
        "general.architecture".to_string(),
        MetadataValue::String("test-arch".to_string()),
    );
    metadata.insert(
        "general.name".to_string(),
        MetadataValue::String("test-model".to_string()),
    );
    metadata.insert("llm.context_length".to_string(), MetadataValue::U32(512));

    // 4 × 8 F32 tensor = 128 bytes
    let shape = TensorShape::new(vec![4, 8]).expect("valid shape");
    let data = TensorData::new_owned(vec![0u8; 128]);
    let tensor = TensorInfo::new(
        "token_embd.weight".to_string(),
        shape,
        GGUFTensorType::F32,
        0,
    );
    let tensors: Vec<(TensorInfo, TensorData)> = vec![(tensor, data)];

    let tmp = NamedTempFile::new().expect("temp file");
    let path = tmp.path().to_path_buf();

    let mut buf = Vec::<u8>::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut writer = GGUFFileWriter::new(BufWriter::new(cursor));
        writer
            .write_complete_file(&metadata, &tensors)
            .expect("write fixture GGUF");
    }

    std::fs::write(&path, &buf).expect("persist fixture");
    (tmp, path)
}

use crate::algorithm::{HashAlgorithm, algorithm_by_name};
use crate::error::HashError;
use crate::format::OutputFormat;
use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const DEFAULT_MMAP_THRESHOLD: u64 = 1 << 30;

pub struct HashEngine {
    mmap_threshold: u64,
}

impl Default for HashEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HashEngine {
    pub fn new() -> Self {
        Self {
            mmap_threshold: DEFAULT_MMAP_THRESHOLD,
        }
    }

    pub fn with_mmap_threshold(bytes: u64) -> Self {
        Self {
            mmap_threshold: bytes,
        }
    }

    pub fn hash_file(
        &self,
        path: &Path,
        alg: &dyn HashAlgorithm,
        format: OutputFormat,
    ) -> Result<String, HashError> {
        let metadata = std::fs::metadata(path).map_err(|e| classify(path, e))?;
        if !metadata.is_file() {
            return Err(HashError::NotAFile {
                path: path.to_path_buf(),
            });
        }
        let digest = self.digest(path, metadata.len(), alg)?;
        Ok(format.render(&digest))
    }

    pub fn hash_file_named(
        &self,
        path: &Path,
        alg_name: &str,
        format: OutputFormat,
    ) -> Result<String, HashError> {
        let alg = algorithm_by_name(alg_name).ok_or_else(|| {
            HashError::Io(std::io::Error::other(format!(
                "unknown algorithm: {alg_name}"
            )))
        })?;
        self.hash_file(path, alg, format)
    }

    pub fn hash_files<'a>(
        &self,
        paths: &'a [PathBuf],
        alg: &(dyn HashAlgorithm + Sync),
        format: OutputFormat,
    ) -> Vec<(&'a PathBuf, Result<String, HashError>)> {
        paths
            .par_iter()
            .map(|p| (p, self.hash_file(p, alg, format)))
            .collect()
    }

    fn digest(&self, path: &Path, len: u64, alg: &dyn HashAlgorithm) -> Result<Vec<u8>, HashError> {
        if len <= self.mmap_threshold {
            if let Some(map) = map_file(path) {
                return alg.digest_reader(&mut &map[..]);
            }
        }
        let mut file = File::open(path).map_err(|e| classify(path, e))?;
        alg.digest_reader(&mut file)
    }
}

fn map_file(path: &Path) -> Option<Mmap> {
    let file = File::open(path).ok()?;
    unsafe { Mmap::map(&file) }.ok()
}

fn classify(path: &Path, err: std::io::Error) -> HashError {
    match err.kind() {
        ErrorKind::NotFound => HashError::NotFound {
            path: path.to_path_buf(),
        },
        ErrorKind::PermissionDenied => HashError::PermissionDenied {
            path: path.to_path_buf(),
        },
        _ => HashError::Io(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::{Md5, Sha256};
    use crate::format::OutputFormat;
    use std::io::Write;

    fn temp_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("yohash-test-{name}"));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn hashes_small_file_via_mmap() {
        let p = temp_file("small", b"abc");
        let e = HashEngine::new();
        assert_eq!(
            e.hash_file(&p, &Sha256, OutputFormat::LowerHex).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hashes_empty_file() {
        let p = temp_file("empty", b"");
        let e = HashEngine::new();
        assert_eq!(
            e.hash_file(&p, &Md5, OutputFormat::UpperHex).unwrap(),
            "D41D8CD98F00B204E9800998ECF8427E"
        );
    }

    #[test]
    fn streaming_path_matches_mmap_path() {
        let data: Vec<u8> = (0..1024u32).flat_map(|i| i.to_le_bytes()).collect();
        let p = temp_file("stream", &data);
        let mmap_engine = HashEngine::new();
        let stream_engine = HashEngine::with_mmap_threshold(0);
        assert_eq!(
            mmap_engine
                .hash_file(&p, &Sha256, OutputFormat::LowerHex)
                .unwrap(),
            stream_engine
                .hash_file(&p, &Sha256, OutputFormat::LowerHex)
                .unwrap()
        );
    }

    #[test]
    fn missing_file_is_a_clear_error() {
        let p = std::path::PathBuf::from("definitely-not-here-12345");
        let e = HashEngine::new();
        assert!(matches!(
            e.hash_file(&p, &Md5, OutputFormat::UpperHex),
            Err(HashError::NotFound { .. })
        ));
    }

    #[test]
    fn directory_input_is_not_a_file_error() {
        let dir = std::env::temp_dir();
        let e = HashEngine::new();
        assert!(matches!(
            e.hash_file(&dir, &Md5, OutputFormat::UpperHex),
            Err(HashError::NotAFile { .. })
        ));
    }

    #[test]
    fn named_lookup_works() {
        let p = temp_file("named", b"abc");
        let e = HashEngine::new();
        assert_eq!(
            e.hash_file_named(&p, "sha256", OutputFormat::LowerHex)
                .unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn multi_file_preserves_input_order() {
        let a = temp_file("order-a", b"a");
        let b = temp_file("order-b", b"b");
        let c = temp_file("order-c", b"c");
        let paths = vec![c.clone(), a.clone(), b.clone()];
        let e = HashEngine::new();
        let out = e.hash_files(&paths, &Sha256, OutputFormat::LowerHex);
        let order: Vec<&PathBuf> = out.iter().map(|(p, _)| *p).collect();
        assert_eq!(order, vec![&c, &a, &b]);
    }

    #[test]
    fn one_failure_does_not_abort_the_batch() {
        let a = temp_file("batch-a", b"a");
        let missing = std::path::PathBuf::from("definitely-not-here-67890");
        let paths = vec![a.clone(), missing.clone()];
        let e = HashEngine::new();
        let out = e.hash_files(&paths, &Md5, OutputFormat::UpperHex);
        assert!(out[0].1.is_ok());
        assert!(matches!(out[1].1, Err(HashError::NotFound { .. })));
    }
}

use crate::error::HashError;
use std::io::Read;

const CHUNK_SIZE: usize = 64 * 1024;

pub trait HashAlgorithm: Send + Sync {
    fn name(&self) -> &'static str;
    fn digest_reader(&self, reader: &mut dyn Read) -> Result<Vec<u8>, HashError>;
}

pub struct Md5;

impl HashAlgorithm for Md5 {
    fn name(&self) -> &'static str {
        "MD5"
    }

    fn digest_reader(&self, reader: &mut dyn Read) -> Result<Vec<u8>, HashError> {
        hash_with::<md5::Md5>(reader)
    }
}

pub struct Sha256;

impl HashAlgorithm for Sha256 {
    fn name(&self) -> &'static str {
        "SHA-256"
    }

    fn digest_reader(&self, reader: &mut dyn Read) -> Result<Vec<u8>, HashError> {
        hash_with::<sha2::Sha256>(reader)
    }
}

pub fn hash_with<D: sha2::digest::Digest>(reader: &mut dyn Read) -> Result<Vec<u8>, HashError> {
    let mut hasher = D::new();
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn digest(alg: &dyn HashAlgorithm, data: &[u8]) -> String {
        let mut r = Cursor::new(data);
        alg.digest_reader(&mut r)
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn md5_empty_vector() {
        assert_eq!(digest(&Md5, b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_abc_vector() {
        assert_eq!(digest(&Md5, b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn sha256_empty_vector() {
        assert_eq!(
            digest(&Sha256, b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc_vector() {
        assert_eq!(
            digest(&Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn names_are_stable() {
        assert_eq!(Md5.name(), "MD5");
        assert_eq!(Sha256.name(), "SHA-256");
    }
}

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("{path}: file not found", path = .path.display())]
    NotFound { path: PathBuf },
    #[error("{path}: permission denied", path = .path.display())]
    PermissionDenied { path: PathBuf },
    #[error("{path}: not a regular file", path = .path.display())]
    NotAFile { path: PathBuf },
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn not_found_message_names_the_path() {
        let e = HashError::NotFound {
            path: PathBuf::from("a.txt"),
        };
        assert!(e.to_string().contains("a.txt"));
    }

    #[test]
    fn not_a_file_message_names_the_path() {
        let e = HashError::NotAFile {
            path: PathBuf::from("dir"),
        };
        assert!(e.to_string().contains("dir"));
    }

    #[test]
    fn io_error_is_convertible_from_std_io_error() {
        let src = std::io::Error::other("boom");
        let e: HashError = src.into();
        assert!(matches!(e, HashError::Io(_)));
    }
}

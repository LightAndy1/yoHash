use base64::Engine as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    UpperHex,
    LowerHex,
    Base64,
}

impl OutputFormat {
    pub fn render(&self, digest: &[u8]) -> String {
        match self {
            OutputFormat::UpperHex => digest.iter().map(|b| format!("{:02X}", b)).collect(),
            OutputFormat::LowerHex => digest.iter().map(|b| format!("{:02x}", b)).collect(),
            OutputFormat::Base64 => base64::engine::general_purpose::STANDARD.encode(digest),
        }
    }

    pub fn parse(s: &str) -> Option<OutputFormat> {
        match s.to_lowercase().as_str() {
            "upper" | "uppercase" | "upperhex" => Some(OutputFormat::UpperHex),
            "lower" | "lowercase" | "lowerhex" => Some(OutputFormat::LowerHex),
            "base64" => Some(OutputFormat::Base64),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_uppercase_hex() {
        assert_eq!(OutputFormat::default(), OutputFormat::UpperHex);
    }

    #[test]
    fn renders_upper_hex() {
        assert_eq!(
            OutputFormat::UpperHex.render(&[0xde, 0xad, 0xbe, 0xef]),
            "DEADBEEF"
        );
    }

    #[test]
    fn renders_lower_hex() {
        assert_eq!(
            OutputFormat::LowerHex.render(&[0xde, 0xad, 0xbe, 0xef]),
            "deadbeef"
        );
    }

    #[test]
    fn renders_base64() {
        assert_eq!(
            OutputFormat::Base64.render(&[0xde, 0xad, 0xbe, 0xef]),
            "3q2+7w=="
        );
    }

    #[test]
    fn parses_aliases_case_insensitively() {
        assert_eq!(OutputFormat::parse("UPPER"), Some(OutputFormat::UpperHex));
        assert_eq!(
            OutputFormat::parse("lowerhex"),
            Some(OutputFormat::LowerHex)
        );
        assert_eq!(OutputFormat::parse("Base64"), Some(OutputFormat::Base64));
        assert_eq!(OutputFormat::parse("nope"), None);
    }
}

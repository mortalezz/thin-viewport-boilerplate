//! session.rs — Stateless Cookie Codec (v2_ zlib + base64url).
//! Exact parity with Python implementation, bounded decompression limit.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

pub const COOKIE_NAME: &str = "tv_policies";
pub const MAX_DECOMPRESSED_BYTES: usize = 65536;

pub fn decode_policy_cookie(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }

    if let Some(token) = raw.strip_prefix("v2_") {
        if let Ok(compressed) = URL_SAFE_NO_PAD.decode(token) {
            let mut decoder = ZlibDecoder::new(&compressed[..]);
            let mut buf = Vec::new();
            let mut take = (&mut decoder).take(MAX_DECOMPRESSED_BYTES as u64 + 1);
            if take.read_to_end(&mut buf).is_ok() && buf.len() <= MAX_DECOMPRESSED_BYTES {
                if let Ok(text) = String::from_utf8(buf) {
                    return text
                        .lines()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        }
    } else if let Ok(items) = serde_json::from_str::<Vec<String>>(raw) {
        return items;
    }

    Vec::new()
}

pub fn encode_policy_cookie(policies: &[String]) -> String {
    let text = policies.join("\n");
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    let _ = encoder.write_all(text.as_bytes());
    let compressed = encoder.finish().unwrap_or_default();
    let token = URL_SAFE_NO_PAD.encode(&compressed);
    format!("v2_{}", token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_roundtrip() {
        let original = vec![
            "POL-NET-01".to_string(),
            "POL-NET-02".to_string(),
            "POL-IAM-01".to_string(),
        ];
        let token = encode_policy_cookie(&original);
        assert!(token.starts_with("v2_"));
        let decoded = decode_policy_cookie(&token);
        assert_eq!(decoded, original);
    }
}

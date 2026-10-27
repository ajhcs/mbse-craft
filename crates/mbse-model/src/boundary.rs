use std::path::Path;
use sysmlv2_syntax::{lexer::tokenize, token::TokenKind};

pub const MAX_SOURCE: usize = 1024 * 1024;
pub const MAX_TOKENS: usize = 100_000;
pub const MAX_ELEMENTS: usize = 10_000;
pub const MAX_DEPTH: usize = 64;
pub const MAX_WIRE: u64 = 16 * 1024 * 1024;

/// Logical identity, not a filesystem path; adapters never follow references.
pub fn source_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || key.len() > 256
        || key.contains(['\\', ':'])
        || key.chars().any(char::is_control)
        || key
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err("UNSAFE_SOURCE_KEY: use a relative POSIX logical key without traversal".into());
    }
    if !key.ends_with(".sysml") {
        return Err("UNSUPPORTED_FORMAT: only .sysml text is implemented; XML and archives are rejected without parsing or extraction".into());
    }
    Ok(())
}

pub fn read_source(path: &Path) -> Result<Vec<u8>, String> {
    crate::local_input::read(path, MAX_SOURCE, "SOURCE").map_err(|error| {
        if error.starts_with("SOURCE_FILE_LIMIT") {
            format!("SOURCE_TOO_LARGE: maximum {MAX_SOURCE} bytes")
        } else {
            error
        }
    })
}

pub fn preflight(key: &str, bytes: &[u8]) -> Result<String, String> {
    source_key(key)?;
    if bytes.len() > MAX_SOURCE {
        return Err("SOURCE_TOO_LARGE".into());
    }
    if bytes.starts_with(b"PK") || bytes.starts_with(&[0x1f, 0x8b]) || bytes.starts_with(b"7z") {
        return Err("UNSUPPORTED_ARCHIVE: no decompression or extraction".into());
    }
    let src = std::str::from_utf8(bytes).map_err(|_| "INVALID_UTF8")?;
    if src
        .trim_start_matches('\u{feff}')
        .trim_start()
        .starts_with('<')
        || src.contains("<!DOCTYPE")
    {
        return Err("UNSUPPORTED_XML: XML is never parsed; external entities and archive paths are never followed".into());
    }
    let (tokens, errors) = tokenize(src);
    if !errors.is_empty() {
        return Err(format!("LEXICAL_ERROR: {}", errors[0]));
    }
    if tokens.len() > MAX_TOKENS {
        return Err("TOKEN_LIMIT".into());
    }
    let mut depth = 0usize;
    let mut segment = 0usize;
    for token in &tokens {
        if token.kind.is_trivia() || token.kind == TokenKind::RegularComment {
            continue;
        }
        segment += 1;
        if segment > 512 {
            return Err("EXPRESSION_WORK_LIMIT: maximum 512 tokens between delimiters".into());
        }
        match token.kind {
            TokenKind::LBrace | TokenKind::LBracket | TokenKind::LParen => {
                depth += 1;
                segment = 0;
                if depth > MAX_DEPTH {
                    return Err("NESTING_LIMIT".into());
                }
            }
            TokenKind::RBrace | TokenKind::RBracket | TokenKind::RParen => {
                depth = depth.saturating_sub(1);
                segment = 0;
            }
            TokenKind::Semi => segment = 0,
            _ => {}
        }
    }
    Ok(src.into())
}

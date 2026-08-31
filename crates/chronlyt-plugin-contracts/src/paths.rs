use crate::{ContractError, ContractResult, limits::MAX_PATH_BYTES};

pub fn validate_relative_path(value: &str) -> ContractResult<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.starts_with('/')
        && !value.contains('\\')
        && value.split('/').all(is_portable_path_segment);
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidData("package path"))
    }
}

fn is_portable_path_segment(part: &str) -> bool {
    let bytes = part.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return false;
    }
    let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
    !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0')
}

//! Protocol versioning and compatibility validation.

/// Current authoritative protocol version implemented by NebuDCS.
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Validates whether a client's requested protocol version is compatible with this server.
///
/// NebuDCS follows Semantic Versioning 2.0.0 for its protocol:
/// - Major version must match exactly (breaking changes).
/// - Minor and patch differences are backward compatible if server >= client.
pub fn is_compatible_protocol_version(client_version: &str) -> bool {
    let parse_semver = |v: &str| -> Option<(u64, u64, u64)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some((major, minor, patch))
    };

    let Some((s_major, s_minor, _)) = parse_semver(PROTOCOL_VERSION) else {
        return false;
    };
    let Some((c_major, c_minor, _)) = parse_semver(client_version) else {
        return false;
    };

    // Major version must be identical
    if s_major != c_major {
        return false;
    }

    // Client cannot request a newer minor version than what this server provides
    c_minor <= s_minor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        assert!(is_compatible_protocol_version("1.0.0"));
        assert!(is_compatible_protocol_version("1.0.1"));
        assert!(!is_compatible_protocol_version("2.0.0"));
        assert!(!is_compatible_protocol_version("0.9.0"));
        assert!(!is_compatible_protocol_version("invalid"));
    }
}

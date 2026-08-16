//! Tiny auth helper used as a Tree-sitter / search fixture.

/// Returns true when `token` is non-empty and does not contain whitespace.
pub fn token_is_well_formed(token: &str) -> bool {
    !token.is_empty() && !token.chars().any(char::is_whitespace)
}

/// Rotates a session identifier by appending a generation counter.
pub fn rotate_session(session_id: &str, generation: u32) -> String {
    format!("{session_id}:{generation}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_token() {
        assert!(!token_is_well_formed(""));
    }

    #[test]
    fn accepts_opaque_token() {
        assert!(token_is_well_formed("abc.def"));
    }

    #[test]
    fn rotation_appends_generation() {
        assert_eq!(rotate_session("sess", 2), "sess:2");
    }
}

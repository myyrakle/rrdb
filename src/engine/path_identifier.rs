//! Validation for SQL identifiers that become filesystem path components.
//!
//! Database, table and index names are joined onto the data directory to form
//! real paths. A quoted identifier may legally contain any character, so
//! without a check `CREATE INDEX "../../../evil"` resolves outside the data
//! directory and `PathBuf::join` will happily build that path (see #255).
//!
//! The check rejects rather than rewrites. Sanitising -- stripping separators
//! or collapsing `..` -- would silently fold distinct identifiers onto the same
//! file, so two different tables could end up sharing storage.
//!
//! Note the platform difference behind the colon rule. On Windows,
//! `Path::join` treats a drive-relative name as absolute and discards the base
//! entirely -- `base_dir.join("C:escape")` is `C:escape`, not
//! `base_dir\C:escape`. Verified on CI rather than assumed: a probe asserting
//! the opposite passed on macOS and Linux and failed on windows-latest.

use crate::errors;
use crate::errors::execute_error::ExecuteError;

/// Reject an identifier that cannot safely be used as a single path component.
///
/// `kind` names the identifier in the error message ("database name", ...).
pub fn validate_path_identifier(name: &str, kind: &str) -> errors::Result<()> {
    let reason = if name.is_empty() {
        "it is empty"
    } else if name == "." || name == ".." {
        "it refers to a directory rather than a name"
    } else if name.contains('/') || name.contains('\\') {
        "it contains a path separator"
    } else if name.contains(':') {
        // Windows reads this as a drive-relative path and drops the base.
        "it contains a drive separator"
    } else if name.contains('\0') {
        "it contains a null byte"
    } else {
        return Ok(());
    };

    Err(ExecuteError::wrap(format!(
        "invalid {}: {:?} cannot be used as a path component because {}",
        kind, name, reason
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_identifiers() {
        // The guard rail: rejecting more is only a fix if everything
        // legitimate still passes. `..` is only refused on its own, so names
        // that merely contain dots stay usable.
        for name in [
            "foo",
            "rrdb",
            "my_table",
            "Table1",
            "테이블",
            "a.b",
            "..leading",
            "trailing..",
            "a..b",
            "-",
            " spaced name ",
        ] {
            assert!(
                validate_path_identifier(name, "table name").is_ok(),
                "{:?} should be accepted",
                name
            );
        }
    }

    #[test]
    fn rejects_names_that_escape_or_confuse_the_path() {
        for name in [
            "",
            ".",
            "..",
            "../evil",
            "../../../etc/passwd",
            "a/b",
            "a\\b",
            "/absolute",
            "with\0null",
            "C:escape",
            "a:b",
        ] {
            assert!(
                validate_path_identifier(name, "table name").is_err(),
                "{:?} should be rejected",
                name
            );
        }
    }

    /// Regression for the colon rule. This started as a probe asserting that
    /// drive-relative names stay inside the base; it passed on macOS and Linux
    /// and *failed* on windows-latest, which is how the rule was confirmed to
    /// be needed rather than assumed. It now asserts the rejection instead.
    #[test]
    fn rejects_windows_drive_relative_names() {
        for name in ["C:escape", "C:\\abs", "a:b", "::"] {
            assert!(
                validate_path_identifier(name, "table name").is_err(),
                "{:?} must be rejected: on Windows base.join({:?}) discards the base entirely",
                name,
                name
            );
        }
    }

    #[test]
    fn the_error_names_the_identifier_and_the_reason() {
        let error = validate_path_identifier("../evil", "index name")
            .expect_err("a traversing name must be rejected");
        let message = error.to_string();
        assert!(message.contains("index name"), "got: {}", message);
        assert!(message.contains("../evil"), "got: {}", message);
        assert!(message.contains("path separator"), "got: {}", message);
    }
}

//! How unresolved policy exceptions are handled at export / batch time.

use serde::{Deserialize, Serialize};

/// Exception handling mode for factory jobs.
///
/// - **Safe** (default): high-confidence auto-cuts apply; pending exceptions are **kept**
///   (not cut). Export proceeds; manifest reports pending count.
/// - **Supervised**: if any exception is pending, the job is **not** exported until resolved.
/// - **Aggressive**: pending exceptions are force-accepted as cuts (explicit consent required).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionHandlingMode {
    #[default]
    Safe,
    Supervised,
    Aggressive,
}

impl ExceptionHandlingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Supervised => "supervised",
            Self::Aggressive => "aggressive",
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::Safe => "Seguro (conserva dudas)",
            Self::Supervised => "Supervisado (revisar antes de exportar)",
            Self::Aggressive => "Agresivo (corta dudas — requiere confirmación)",
        }
    }

    /// Map legacy `auto_accept_exceptions` boolean.
    pub fn from_auto_accept(auto_accept: bool) -> Self {
        if auto_accept {
            Self::Aggressive
        } else {
            Self::Safe
        }
    }

    pub fn is_aggressive(self) -> bool {
        matches!(self, Self::Aggressive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_safe() {
        assert_eq!(
            ExceptionHandlingMode::default(),
            ExceptionHandlingMode::Safe
        );
    }

    #[test]
    fn legacy_mapping() {
        assert_eq!(
            ExceptionHandlingMode::from_auto_accept(true),
            ExceptionHandlingMode::Aggressive
        );
        assert_eq!(
            ExceptionHandlingMode::from_auto_accept(false),
            ExceptionHandlingMode::Safe
        );
    }
}

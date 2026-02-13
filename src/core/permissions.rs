use serde::{Deserialize, Serialize};

/// Difficulty levels control what operations kissa will perform (ADR-500).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DifficultyLevel {
    Readonly,
    Fetch,
    Commit,
    Force,
    Unsafe,
}

impl DifficultyLevel {
    pub fn display_name(&self, cat_mode: bool) -> &'static str {
        if cat_mode {
            match self {
                Self::Readonly => "napping",
                Self::Fetch => "purring",
                Self::Commit => "hunting",
                Self::Force => "zoomies",
                Self::Unsafe => "knocking-things-off-the-counter",
            }
        } else {
            match self {
                Self::Readonly => "readonly",
                Self::Fetch => "fetch",
                Self::Commit => "commit",
                Self::Force => "force",
                Self::Unsafe => "unsafe",
            }
        }
    }
}

/// An operation category that maps to a minimum difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    Read,
    Fetch,
    Write,
    Force,
    Destructive,
}

impl OperationClass {
    /// Minimum difficulty level required for this operation class.
    pub fn required_level(&self) -> DifficultyLevel {
        match self {
            Self::Read => DifficultyLevel::Readonly,
            Self::Fetch => DifficultyLevel::Fetch,
            Self::Write => DifficultyLevel::Commit,
            Self::Force => DifficultyLevel::Force,
            Self::Destructive => DifficultyLevel::Unsafe,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_ordering() {
        assert!(DifficultyLevel::Readonly < DifficultyLevel::Commit);
        assert!(DifficultyLevel::Commit < DifficultyLevel::Force);
        assert!(DifficultyLevel::Force < DifficultyLevel::Unsafe);
    }

    #[test]
    fn operation_class_levels() {
        assert_eq!(OperationClass::Read.required_level(), DifficultyLevel::Readonly);
        assert_eq!(OperationClass::Write.required_level(), DifficultyLevel::Commit);
        assert_eq!(OperationClass::Destructive.required_level(), DifficultyLevel::Unsafe);
    }

    #[test]
    fn cat_mode_names() {
        assert_eq!(DifficultyLevel::Readonly.display_name(true), "napping");
        assert_eq!(DifficultyLevel::Unsafe.display_name(true), "knocking-things-off-the-counter");
        assert_eq!(DifficultyLevel::Commit.display_name(false), "commit");
    }
}

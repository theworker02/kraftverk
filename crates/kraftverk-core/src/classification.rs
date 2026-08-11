//! Comparison classification labels.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComparisonClass {
    ConfirmedImprovement,
    LikelyImprovement,
    NoSignificantChange,
    LikelyRegression,
    ConfirmedRegression,
    UnstableResult,
}

impl ComparisonClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfirmedImprovement => "CONFIRMED_IMPROVEMENT",
            Self::LikelyImprovement => "LIKELY_IMPROVEMENT",
            Self::NoSignificantChange => "NO_SIGNIFICANT_CHANGE",
            Self::LikelyRegression => "LIKELY_REGRESSION",
            Self::ConfirmedRegression => "CONFIRMED_REGRESSION",
            Self::UnstableResult => "UNSTABLE_RESULT",
        }
    }

    pub fn is_improvement(self) -> bool {
        matches!(self, Self::ConfirmedImprovement | Self::LikelyImprovement)
    }

    pub fn is_regression(self) -> bool {
        matches!(self, Self::ConfirmedRegression | Self::LikelyRegression)
    }
}

impl std::fmt::Display for ComparisonClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

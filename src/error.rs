use thiserror::Error;

/// Application-level errors for the flight tracker.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to parse response: {0}")]
    Parse(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

impl AppError {
    /// Returns a user-friendly error message suitable for display in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::RateLimited => "API rate limit reached. Try again later.".to_string(),
            Self::Network(_) => "Network error. Check your connection.".to_string(),
            Self::Parse(_) => "Failed to parse flight data.".to_string(),
        }
    }
}

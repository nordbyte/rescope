use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RescopeError {
    #[error("invalid duration \"{input}\"; examples: 30s, 1m, 5m")]
    InvalidDuration { input: String },

    #[error("duration must be greater than or equal to interval")]
    DurationShorterThanInterval,

    #[error("interval must be at least {minimum_ms}ms")]
    IntervalTooShort { minimum_ms: u128 },

    #[error("limit must be greater than 0")]
    InvalidLimit,

    #[error("sampling error: {0}")]
    Sampling(String),

    #[error("no samples were collected")]
    NoSamples,

    #[error("time conversion failed")]
    TimeConversion,
}

pub fn validate_recording_timing(
    duration: Duration,
    interval: Duration,
    minimum_interval: Duration,
) -> Result<(), RescopeError> {
    if interval < minimum_interval {
        return Err(RescopeError::IntervalTooShort {
            minimum_ms: minimum_interval.as_millis(),
        });
    }

    if duration < interval {
        return Err(RescopeError::DurationShorterThanInterval);
    }

    Ok(())
}

pub fn validate_interval(
    interval: Duration,
    minimum_interval: Duration,
) -> Result<(), RescopeError> {
    if interval < minimum_interval {
        return Err(RescopeError::IntervalTooShort {
            minimum_ms: minimum_interval.as_millis(),
        });
    }

    Ok(())
}

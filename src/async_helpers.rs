//! Async I/O helpers for procfs2.
//!
//! This module provides async variants of file reading operations using tokio.
//! These are useful when integrating procfs2 with async applications.
//!
//! # Usage
//!
//! Requires the `async` feature to be enabled.
//!
//! ```ignore
//! use procfs2::async_helpers;
//!
//! let content = async_helpers::read_file("/proc/meminfo").await?;
//! ```

use std::path::Path;
use std::time::Duration;
use std::{future::Future, io};

use crate::error::{Error, Result};

/// Asynchronously reads the entire contents of a file into a vector of bytes.
///
/// This is the async counterpart to `std::fs::read()` and uses tokio's
/// async file I/O operations under the hood.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub async fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    use tokio::fs;

    let path = path.as_ref().to_path_buf();
    fs::read(&path).await.map_err(|e| crate::error::Error::Io {
        path: Some(path),
        error: e,
    })
}

/// Asynchronously reads the entire contents of a file into a string.
///
/// This is the async counterpart to `std::fs::read_to_string()` and uses
/// tokio's async file I/O operations.
///
/// # Arguments
///
/// * `path` - The path to the file to read
///
/// # Errors
///
/// Returns an error if the file cannot be read or is not valid UTF-8.
pub async fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    use tokio::fs;

    let path = path.as_ref().to_path_buf();
    fs::read_to_string(&path)
        .await
        .map_err(|e| crate::error::Error::Io {
            path: Some(path),
            error: e,
        })
}

/// Polls an asynchronous snapshot function and yields each result.
///
/// Polling starts immediately. If the callback returns an error, the stream
/// yields it and ends.
pub fn watch<T, F, Fut>(
    period: Duration,
    mut snapshot: F,
) -> impl futures_core::Stream<Item = Result<T>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    async_stream::stream! {
        if period.is_zero() {
            yield Err(Error::Io {
                path: None,
                error: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "polling period must be non-zero",
                ),
            });
            return;
        }

        let mut ticker = tokio::time::interval(period);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match snapshot().await {
                Ok(value) => yield Ok(value),
                Err(error) => {
                    yield Err(error);
                    return;
                }
            }
        }
    }
}

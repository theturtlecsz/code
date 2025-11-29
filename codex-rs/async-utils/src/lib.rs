//! Async utilities for cancellation-aware futures.
//!
//! Provides the `OrCancelExt` trait for making futures cancellable
//! with tokio's `CancellationToken`.

use async_trait::async_trait;
use std::future::Future;
use tokio_util::sync::CancellationToken;

/// Error returned when a future is cancelled.
#[derive(Debug, PartialEq, Eq)]
pub enum CancelErr {
    Cancelled,
}

/// Extension trait for making futures cancellable.
///
/// Allows any future to race against a `CancellationToken`, returning
/// `Err(CancelErr::Cancelled)` if the token is cancelled before the
/// future completes.
#[async_trait]
pub trait OrCancelExt: Sized {
    type Output;

    /// Race this future against the cancellation token.
    ///
    /// Returns `Ok(output)` if the future completes first, or
    /// `Err(CancelErr::Cancelled)` if the token is cancelled.
    async fn or_cancel(self, token: &CancellationToken) -> Result<Self::Output, CancelErr>;
}

#[async_trait]
impl<F> OrCancelExt for F
where
    F: Future + Send,
    F::Output: Send,
{
    type Output = F::Output;

    async fn or_cancel(self, token: &CancellationToken) -> Result<Self::Output, CancelErr> {
        tokio::select! {
            _ = token.cancelled() => Err(CancelErr::Cancelled),
            res = self => Ok(res),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::time::Duration;
    use tokio::task;
    use tokio::time::sleep;

    #[tokio::test]
    async fn returns_ok_when_future_completes_first() {
        let token = CancellationToken::new();
        let value = async { 42 };

        let result = value.or_cancel(&token).await;

        assert_eq!(Ok(42), result);
    }

    #[tokio::test]
    async fn returns_err_when_token_cancelled_first() {
        let token = CancellationToken::new();
        let token_clone = token.clone();

        let cancel_handle = task::spawn(async move {
            sleep(Duration::from_millis(10)).await;
            token_clone.cancel();
        });

        let result = async {
            sleep(Duration::from_millis(100)).await;
            7
        }
        .or_cancel(&token)
        .await;

        cancel_handle.await.expect("cancel task panicked");
        assert_eq!(Err(CancelErr::Cancelled), result);
    }

    #[tokio::test]
    async fn returns_err_when_token_already_cancelled() {
        let token = CancellationToken::new();
        token.cancel();

        let result = async {
            sleep(Duration::from_millis(50)).await;
            5
        }
        .or_cancel(&token)
        .await;

        assert_eq!(Err(CancelErr::Cancelled), result);
    }
}

use super::error::ServiceError;
use tokio::sync::oneshot;

/// Message sent to the batch worker
#[derive(Debug)]
pub(crate) struct Message<Request, Fut> {
    pub(crate) request: Request,
    pub(crate) tx: Tx<Fut>,
    pub(crate) span: tracing::Span,
    pub(super) _permit: crate::semaphore::Permit,
}

/// Response sender
pub(crate) type Tx<Fut> = oneshot::Sender<Result<Fut, ServiceError>>;

/// Response receiver
pub(crate) type Rx<Fut> = oneshot::Receiver<Result<Fut, ServiceError>>;

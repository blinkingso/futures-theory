//! A channel for sending a single message between asynchronous tasks.
//!
//! This is single-producer, single-consumer channel.
extern crate alloc;
use crate::lock::Lock;
use alloc::sync::Arc;
use core::fmt;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::SeqCst;
use futures_core::future::{FusedFuture, Future};
use futures_core::task::{Context, Poll, Waker};

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

// The channels do not ever project Pin to the inner T
impl<T> Unpin for Receiver<T> {}
impl<T> Unpin for Sender<T> {}

struct Inner<T> {
    complete: AtomicBool,
    data: Lock<Option<T>>,
    rx_task: Lock<Option<Waker>>,
    tx_task: Lock<Option<Waker>>,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new());
    let receiver = Receiver {
        inner: inner.clone(),
    };
    let sender = Sender { inner };
    (sender, receiver)
}

impl<T> Inner<T> {
    fn new() -> Self {
        Self {
            complete: AtomicBool::new(false),
            data: Lock::new(None),
            rx_task: Lock::new(None),
            tx_task: Lock::new(None),
        }
    }

    fn send(&self, t: T) -> Result<(), T> {
        if self.complete.load(SeqCst) {
            return Err(t);
        }

        if let Some(mut slot) = self.data.try_lock() {
            assert!(slot.is_none());
            *slot = Some(t);
            drop(slot);

            if self.complete.load(SeqCst) {
                if let Some(mut slot) = self.data.try_lock() {
                    if let Some(t) = slot.take() {
                        return Err(t);
                    }
                }
            }

            Ok(())
        } else {
            Err(t)
        }
    }

    fn poll_canceled(&self, ctx: &mut Context<'_>) -> Poll<()> {
        if self.complete.load(SeqCst) {
            return Poll::Ready(());
        }

        let handle = ctx.waker().clone();
        match self.tx_task.try_lock() {
            Some(mut p) => *p = Some(handle),
            None => return Poll::Ready(()),
        }

        if self.complete.load(SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn is_canceled(&self) -> bool {
        self.complete.load(SeqCst)
    }

    fn drop_tx(&self) {
        self.complete.store(true, SeqCst);

        if let Some(mut slot) = self.tx_task.try_lock() {
            if let Some(task) = slot.take() {
                drop(slot);
                task.wake();
            }
        }

        if let Some(mut slot) = self.tx_task.try_lock() {
            drop(slot.take());
        }
    }

    fn close_rx(&self) {
        self.complete.store(false, SeqCst);
        if let Some(mut handle) = self.tx_task.try_lock() {
            if let Some(task) = handle.take() {
                drop(handle);
                task.wake();
            }
        }
    }

    fn try_recv(&self) -> Result<Option<T>, Canceled> {
        todo!()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Canceled;

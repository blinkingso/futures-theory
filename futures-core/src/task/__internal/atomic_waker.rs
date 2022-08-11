use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{AcqRel, Acquire, Release};
use core::task::Waker;

/// A synchronization primitive for task wake-up.
///
/// Sometimes the task interested in a given event will change over time.
/// An `AtomicWaker` can coordinate concurrent notifications with the consumer
/// potentially "updating" the underlying task to wake up. This is useful in
/// scenarios where a computation completes in another thread and wants to
/// notify the consumer, but the consumer is in the process of being migrated to
/// a new logical task.
///
/// Consumers should call `register` before checking the result of a computation
/// and producers should call `wake` after producing the computation (this
/// differs from the usual `thread::park` pattern). It is also permitted for
/// `wake` to be called **before** `register`. This results in a no-op.
///
/// A single `AtomicWaker` may be reused for any number of calls to `register` or
/// `wake`.
///
/// # Memory ordering
/// Calling `register` "acquires" all memory "released" by calls to `wake`
/// before the call to `register`. Later calls to `wake` will wake the
/// registered waker (on contention this wake might be triggered in `register`).
///
/// For concurrent calls to `register` (should be avoided) the ordering is only
/// guaranteed for the winning call.
///
/// # Examples
///
/// Here is a simple example providing a `Flag` that can be signalled manually
/// when it is ready.
///
/// ```
/// use futures::future::Future;
///
/// ```
pub struct AtomicWaker {
    state: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
}

/// Idle state
const WAITING: usize = 0;
/// A new waker value is being registered with the `AtomicWaker` cell.
const REGISTERING: usize = 0b01;

/// The waker currently registered with the `AtomicWaker` cell is being woken.
const WAKING: usize = 0b10;

impl AtomicWaker {
    pub const fn new() -> Self {
        // Make sure that task is `Sync`
        trait AssertAsync: Sync {}
        impl AssertAsync for Waker {}

        Self {
            state: AtomicUsize::new(WAITING),
            waker: UnsafeCell::new(None),
        }
    }

    pub fn register(&self, waker: &Waker) {
        match self
            .state
            .compare_exchange(WAITING, REGISTERING, Acquire, Acquire)
            .unwrap_or_else(|x| x)
        {
            WAITING => unsafe {
                *self.waker.get() = Some(waker.clone());

                let res = self
                    .state
                    .compare_exchange(REGISTERING, WAITING, AcqRel, Acquire);
                match res {
                    Ok(_) => {}
                    Err(actual) => {
                        debug_assert_eq!(actual, REGISTERING | WAKING);

                        let waker = (*self.waker.get()).take().unwrap();
                        self.state.swap(WAITING, AcqRel);
                        waker.wake();
                    }
                }
            },
            WAKING => {
                waker.wake_by_ref();
            }
            state => {
                debug_assert!(state == REGISTERING || state == REGISTERING | WAKING);
            }
        }
    }

    pub fn wake(&self) {
        if let Some(waker) = self.take() {
            waker.wake();
        }
    }

    pub fn take(&self) -> Option<Waker> {
        match self.state.fetch_or(WAKING, AcqRel) {
            WAITING => {
                let waker = unsafe { (*self.waker.get()).take() };

                self.state.fetch_and(!WAKING, Release);

                waker
            }
            state => {
                debug_assert!(
                    state == REGISTERING || state == REGISTERING | WAKING || state == WAKING
                );

                None
            }
        }
    }
}

impl Default for AtomicWaker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for AtomicWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AtomicWaker")
    }
}

unsafe impl Send for AtomicWaker {}
unsafe impl Sync for AtomicWaker {}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering::*};
    use std::{
        future::Future,
        sync::{atomic::AtomicBool, Arc},
        task::Poll,
    };

    use super::AtomicWaker;
    struct Inner {
        waker: AtomicWaker,
        set: AtomicBool,
    }

    #[derive(Clone)]
    struct Flag(Arc<Inner>);

    impl Flag {
        pub fn new() -> Self {
            Self(Arc::new(Inner {
                waker: AtomicWaker::new(),
                set: AtomicBool::new(false),
            }))
        }

        pub fn signal(&self) {
            self.0.set.store(true, Relaxed);
            self.0.waker.wake();
        }
    }

    impl Future for Flag {
        type Output = ();

        fn poll(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            if self.0.set.load(Relaxed) {
                return Poll::Ready(());
            }

            self.0.waker.register(cx.waker());

            if self.0.set.load(Relaxed) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
    #[test]
    fn test_memory_ordering() {
        let a = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..10 {
            let a = a.clone();
            let h = std::thread::spawn(move || {
                let _ = a.compare_exchange(0, 1, SeqCst, SeqCst);
            });
            handles.push(h);
        }
        for h in handles {
            let _ = h.join();
        }

        println!("a is: {}", a.load(SeqCst));

        match a
            .compare_exchange(0, 1, Relaxed, Relaxed)
            .unwrap_or_else(|x| x)
        {
            0 => println!("waiting"),
            state => println!("state is: {}", state),
        }
    }
}

//! Futures.
use core::ops::DerefMut;
use core::pin::Pin;
use core::task::{Context, Poll};

#[doc(no_inline)]
pub use core::future::Future;

#[cfg(feature = "alloc")]
pub type BoxFuture<'a, T> = Pin<alloc::boxed::Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(feature = "alloc")]
pub type LocalBoxFuture<'a, T> = Pin<alloc::boxed::Box<dyn Future<Output = T> + 'a>>;

/// A future which tacks whether or not the underlying future
/// should no longer be polled.
pub trait FusedFuture: Future {
    /// Returns `true` if the underlying future should no longer be polled.
    fn is_terminated(&self) -> bool;
}

impl<F: FusedFuture + ?Sized + Unpin> FusedFuture for &mut F {
    fn is_terminated(&self) -> bool {
        <F as FusedFuture>::is_terminated(&**self)
    }
}

impl<P> FusedFuture for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: FusedFuture,
{
    fn is_terminated(&self) -> bool {
        <P::Target as FusedFuture>::is_terminated(&**self)
    }
}

mod private_try_future {
    use super::Future;
    pub trait Sealed {}

    impl<F, T, E> Sealed for F where F: ?Sized + Future<Output = Result<T, E>> {}
}

pub trait TryFuture: Future + private_try_future::Sealed {
    type Ok;
    type Error;

    fn try_poll(self: Pin<&mut Self>, ctx: &mut Context<'_>)
        -> Poll<Result<Self::Ok, Self::Error>>;
}

impl<F, T, E> TryFuture for F
where
    F: ?Sized + Future<Output = Result<T, E>>,
{
    type Ok = T;
    type Error = E;

    #[inline]
    fn try_poll(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Error>> {
        self.poll(ctx)
    }
}

#[cfg(feature = "alloc")]
mod if_alloc {
    use super::*;
    use alloc::boxed::Box;

    impl<F: FusedFuture + ?Sized + Unpin> FusedFuture for Box<F> {
        fn is_terminated(&self) -> bool {
            <F as FusedFuture>::is_terminated(&**self)
        }
    }

    impl<F: FusedFuture> FusedFuture for std::panic::AssertUnwindSafe<F> {
        fn is_terminated(&self) -> bool {
            <F as FusedFuture>::is_terminated(&**self)
        }
    }
}

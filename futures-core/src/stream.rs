//! Asynchronous streams.

use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::ops::DerefMut;

#[cfg(feature = "alloc")]
pub type BoxStream<'a, T> = Pin<alloc::boxed::Box<dyn Stream<Item = T> + Send + 'a>>;
#[cfg(feature = "alloc")]
pub type LocalBoxStream<'a, T> = Pin<alloc::boxed::Box<dyn Stream<Item = T> + 'a>>;

#[must_use = "streams do nothing unless polled"]
pub trait Stream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>>;

    /// Returns the bounds on the remaining length of the stream.
    /// Returns a tuple where the first element is the lower bound, and
    /// the second element is the upper bound.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<S: ?Sized + Stream + Unpin> Stream for &mut S {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        S::poll_next(Pin::new(&mut **self), ctx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
}

impl<P> Stream for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: Stream,
{
    type Item = <P::Target as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().as_mut().poll_next(ctx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
}

pub trait FusedStream: Stream {
    fn is_terminated(&self) -> bool;
}

impl<F: ?Sized + FusedStream + Unpin> FusedStream for &mut F {
    fn is_terminated(&self) -> bool {
        <F as FusedStream>::is_terminated(&**self)
    }
}

impl<P> FusedStream for Pin<P>
where
    P: DerefMut + Unpin,
    P::Target: FusedStream,
{
    fn is_terminated(&self) -> bool {
        <P::Target as FusedStream>::is_terminated(&**self)
    }
}

mod private_try_stream {
    use super::*;
    pub trait Sealed {}

    impl<S, T, E> Sealed for S where S: ?Sized + Stream<Item = Result<T, E>> {}
}

pub trait TryStream: Stream + private_try_stream::Sealed {
    type Ok;
    type Error;

    fn try_poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Ok, Self::Error>>>;
}

impl<S, T, E> TryStream for S
where
    S: ?Sized + Stream<Item = Result<T, E>>,
{
    type Ok = T;
    type Error = E;

    fn try_poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Ok, Self::Error>>> {
        self.poll_next(ctx)
    }
}

#[cfg(feature = "alloc")]
mod if_alloc {
    use super::*;
    use alloc::boxed::Box;

    impl<S: ?Sized + Stream + Unpin> Stream for Box<S> {
        type Item = S::Item;

        fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Pin::new(&mut **self).poll_next(ctx)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (&**self).size_hint()
        }
    }

    #[cfg(feature = "std")]
    impl<S: Stream> Stream for std::panic::AssertUnwindSafe<S> {
        type Item = S::Item;

        fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            unsafe { self.map_unchecked_mut(|x| &mut x.0) }.poll_next(ctx)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            self.0.size_hint()
        }
    }

    impl<S: ?Sized + FusedStream + Unpin> FusedStream for Box<S> {
        fn is_terminated(&self) -> bool {
            <S as FusedStream>::is_terminated(&**self)
        }
    }
}

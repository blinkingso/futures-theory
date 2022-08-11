#[cfg(feature = "std")]
mod if_std {
    use std::io;
    use std::ops::DerefMut;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[allow(unreachable_pub)]
    #[doc(no_inline)]
    pub use io::{IoSlice, IoSliceMut, Result, SeekFrom};

    pub trait AsyncRead {
        fn poll_read(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize>>;

        fn poll_read_vectored(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            bufs: &mut [IoSliceMut<'_>],
        ) -> Poll<Result<usize>> {
            for b in bufs {
                if !b.is_empty() {
                    return self.poll_read(ctx, b);
                }
            }

            self.poll_read(ctx, &mut [])
        }
    }

    pub trait AsyncWrite {
        fn poll_write(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>>;

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize>> {
            for b in bufs {
                if !b.is_empty() {
                    return self.poll_write(ctx, b);
                }
            }

            self.poll_write(ctx, &[])
        }

        fn poll_flush(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>>;

        fn poll_close(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>>;
    }

    pub trait AsyncSeek {
        fn poll_seek(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<Result<u64>>;
    }

    pub trait AsyncBufRead: AsyncRead {
        fn poll_fill_buf(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<&[u8]>>;

        fn consume(self: Pin<&mut Self>, amt: usize);
    }

    macro_rules! deref_async_read {
        () => {
            fn poll_read(
                mut self: Pin<&mut Self>,
                ctx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<Result<usize>> {
                Pin::new(&mut **self).poll_read(ctx, buf)
            }

            fn poll_read_vectored(
                mut self: Pin<&mut Self>,
                ctx: &mut Context<'_>,
                bufs: &mut [IoSliceMut<'_>],
            ) -> Poll<Result<usize>> {
                Pin::new(&mut **self).poll_read_vectored(ctx, bufs)
            }
        };
    }

    impl<T: ?Sized + AsyncRead + Unpin> AsyncRead for Box<T> {
        deref_async_read!();
    }

    impl<T: ?Sized + AsyncRead + Unpin> AsyncRead for &mut T {
        deref_async_read!();
    }

    impl<P> AsyncRead for Pin<P>
    where
        P: DerefMut + Unpin,
        P::Target: AsyncRead,
    {
        fn poll_read(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize>> {
            self.get_mut().as_mut().poll_read(ctx, buf)
        }

        fn poll_read_vectored(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            bufs: &mut [IoSliceMut<'_>],
        ) -> Poll<Result<usize>> {
            self.get_mut().as_mut().poll_read_vectored(ctx, bufs)
        }
    }

    macro_rules! delegate_async_read_to_stdio {
        () => {
            fn poll_read(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<Result<usize>> {
                Poll::Ready(io::Read::read(&mut *self, buf))
            }

            fn poll_read_vectored(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
                bufs: &mut [IoSliceMut<'_>],
            ) -> Poll<Result<usize>> {
                Poll::Ready(io::Read::read_vectored(&mut *self, bufs))
            }
        };
    }

    impl AsyncRead for &[u8] {
        delegate_async_read_to_stdio!();
    }

    macro_rules! deref_async_write {
        () => {
            fn poll_write(
                mut self: Pin<&mut Self>,
                ctx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<Result<usize>> {
                Pin::new(&mut **self).poll_write(ctx, buf)
            }

            fn poll_write_vectored(
                mut self: Pin<&mut Self>,
                ctx: &mut Context<'_>,
                bufs: &[IoSlice<'_>],
            ) -> Poll<Result<usize>> {
                Pin::new(&mut **self).poll_write_vectored(ctx, bufs)
            }

            fn poll_flush(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
                Pin::new(&mut **self).poll_flush(ctx)
            }

            fn poll_close(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
                Pin::new(&mut **self).poll_close(ctx)
            }
        };
    }

    impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for Box<T> {
        deref_async_write!();
    }

    impl<T: ?Sized + AsyncWrite + Unpin> AsyncWrite for &mut T {
        deref_async_write!();
    }

    impl<P> AsyncWrite for Pin<P>
    where
        P: DerefMut + Unpin,
        P::Target: AsyncWrite,
    {
        fn poll_write(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            self.get_mut().as_mut().poll_write(ctx, buf)
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            ctx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize>> {
            self.get_mut().as_mut().poll_write_vectored(ctx, bufs)
        }

        fn poll_flush(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
            self.get_mut().as_mut().poll_flush(ctx)
        }

        fn poll_close(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
            self.get_mut().as_mut().poll_close(ctx)
        }
    }

    macro_rules! delegate_async_write_to_stdio {
        () => {
            fn poll_write(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<Result<usize>> {
                Poll::Ready(io::Write::write(&mut *self, buf))
            }

            fn poll_write_vectored(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
                bufs: &[IoSlice<'_>],
            ) -> Poll<Result<usize>> {
                Poll::Ready(io::Write::write_vectored(&mut *self, bufs))
            }

            fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
                Poll::Ready(io::Write::flush(&mut *self))
            }

            fn poll_close(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<()>> {
                self.poll_flush(ctx)
            }
        };
    }

    impl AsyncWrite for Vec<u8> {
        delegate_async_write_to_stdio!();
    }

    macro_rules! deref_async_seek {
        () => {
            fn poll_seek(
                mut self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                pos: SeekFrom,
            ) -> Poll<Result<u64>> {
                Pin::new(&mut **self).poll_seek(cx, pos)
            }
        };
    }

    impl<T: ?Sized + AsyncSeek + Unpin> AsyncSeek for Box<T> {
        deref_async_seek!();
    }

    impl<T: ?Sized + AsyncSeek + Unpin> AsyncSeek for &mut T {
        deref_async_seek!();
    }

    impl<P> AsyncSeek for Pin<P>
    where
        P: DerefMut + Unpin,
        P::Target: AsyncSeek,
    {
        fn poll_seek(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<Result<u64>> {
            self.get_mut().as_mut().poll_seek(cx, pos)
        }
    }

    macro_rules! deref_async_buf_read {
        () => {
            fn poll_fill_buf(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
                Pin::new(&mut **self.get_mut()).poll_fill_buf(ctx)
            }

            fn consume(mut self: Pin<&mut Self>, amt: usize) {
                Pin::new(&mut **self).consume(amt)
            }
        };
    }

    impl<T: ?Sized + AsyncBufRead + Unpin> AsyncBufRead for Box<T> {
        deref_async_buf_read!();
    }

    impl<T: ?Sized + AsyncBufRead + Unpin> AsyncBufRead for &mut T {
        deref_async_buf_read!();
    }

    impl<P> AsyncBufRead for Pin<P>
    where
        P: DerefMut + Unpin,
        P::Target: AsyncBufRead,
    {
        fn poll_fill_buf(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
            self.get_mut().as_mut().poll_fill_buf(ctx)
        }

        fn consume(self: Pin<&mut Self>, amt: usize) {
            self.get_mut().as_mut().consume(amt)
        }
    }

    macro_rules! delegate_async_buf_read_to_stdio {
        () => {
            fn poll_fill_buf(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<&[u8]>> {
                Poll::Ready(io::BufRead::fill_buf(self.get_mut()))
            }

            fn consume(self: Pin<&mut Self>, amt: usize) {
                io::BufRead::consume(self.get_mut(), amt)
            }
        };
    }

    impl AsyncBufRead for &[u8] {
        delegate_async_buf_read_to_stdio!();
    }
}

#[cfg(feature = "std")]
pub use self::if_std::*;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod future;

#[doc(no_inline)]
pub use self::future::{FusedFuture, Future, TryFuture};

pub mod stream;
#[doc(no_inline)]
pub use self::stream::{FusedStream, Stream, TryStream};

#[macro_use]
pub mod task;

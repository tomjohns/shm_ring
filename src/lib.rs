pub mod ringbuffer_ro;
pub mod ringbuffer_rw;
/// This module defines functions to transfer msg's from one ringbuffer to another
pub mod drain_and_fill;
#[cfg(feature = "avx2")]
pub mod avx;

pub const SZ_OF_USIZE: usize = core::mem::size_of::<usize>();
#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]

extern crate alloc;

pub mod change_stream;
pub mod continuous_bool_vec;
pub mod continuous_bool_vec_2;
pub mod insert;
pub mod remove;
pub mod stream_with_initial;
pub mod try_push;

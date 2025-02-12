#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod change_stream;
pub mod continuous_bool_vec;
pub mod insert;
pub mod remove;
pub mod stream_with_initial;

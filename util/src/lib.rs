#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(sync_unsafe_cell)]
#![feature(int_roundings)]
#![feature(non_null_from_ref)]

extern crate alloc;

pub mod aligned_chunks;
pub mod allocator_test;
pub mod bump_allocator;
pub mod change_stream;
pub mod continuous_bool_vec;
pub mod continuous_bool_vec_2;
pub mod get_table_indexes;
pub mod init_later;
pub mod insert;
#[cfg(test)]
pub mod mock_memory;
pub mod paging_allocator;
pub mod remove;
pub mod stream_with_initial;
pub mod traverse_page_tables;
pub mod try_push;
pub mod usable_frames_iterator;
pub mod virtual_address_from_parts;

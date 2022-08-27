use std::mem::size_of;

// length prefix, two version components
pub const STARTUP_HEADER_SIZE: usize = size_of::<i32>() + (size_of::<i16>() * 2);
// message tag, length prefix
pub const MESSAGE_HEADER_SIZE: usize = size_of::<u8>() + size_of::<i32>();

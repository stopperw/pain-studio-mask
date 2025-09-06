use std::ffi::c_void;

use log::debug;

pub unsafe fn info_write<T>(data: *const T, lp_output: *mut c_void) -> u32 {
    let size = size_of::<T>() as u32;
    debug!("info_write({:#?}, {:#?}) -> {}", data, lp_output, size);
    if lp_output.is_null() {
        return size;
    }
    unsafe {
        std::ptr::copy(data, lp_output as *mut _, 1);
    }
    size
}

pub unsafe fn info_write_array<T>(data: *const T, lp_output: *mut c_void, len: usize) -> u32 {
    let size = size_of::<T>() as u32;
    debug!(
        "info_write_array({:#?}, {:#?}, {}) -> {}",
        data, lp_output, len, size
    );
    if lp_output.is_null() {
        return size;
    }
    unsafe {
        std::ptr::copy(data, lp_output as *mut _, len);
    }
    size
}

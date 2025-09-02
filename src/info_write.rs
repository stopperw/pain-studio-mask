use std::ffi::c_void;

use log::debug;

// pub trait InfoWrite {
//     pub fn info_write(&self, lp_output: *mut c_void) -> u32;
// }
//
// impl InfoWrite for u16 {
//     fn info_write(&self, lp_output: *mut c_void) -> u32 {
//         let size = size_of::<Self>() as u32;
//         unsafe { std::ptr::copy(self, lp_output as *mut _, 1); }
//         size
//     }
// }

pub fn info_write<T>(data: *const T, lp_output: *mut c_void) -> u32 {
// pub fn info_write<T>(data: &T, lp_output: *mut c_void) -> u32 {
    let size = size_of::<T>() as u32;
    debug!("info_write({:#?}, {:#?}) -> {}", data, lp_output, size);
    if lp_output == std::ptr::null_mut() {
        return size;
    }
    unsafe { std::ptr::copy(data, lp_output as *mut _, 1); }
    // unsafe { std::mem::copy(data, lp_output as *mut _, 1); }
    // unsafe { std::ptr::write_unaligned(data, lp_output as *mut _); }
    // unsafe { std::ptr::write_unaligned(lp_output as *mut _, data); }
    size
}

pub fn info_write_force_size<T>(data: *const T, lp_output: *mut c_void, size: u32) -> u32 {
// pub fn info_write<T>(data: &T, lp_output: *mut c_void) -> u32 {
    debug!("info_write_force_size({:#?}, {:#?}, {}) -> {}", data, lp_output, size, size);
    if lp_output == std::ptr::null_mut() {
        return size;
    }
    unsafe { std::ptr::copy(data, lp_output as *mut _, 1); }
    // unsafe { std::mem::copy(data, lp_output as *mut _, 1); }
    // unsafe { std::ptr::write_unaligned(data, lp_output as *mut _); }
    // unsafe { std::ptr::write_unaligned(lp_output as *mut _, data); }
    size
}

pub fn info_write_array<T>(data: *const T, lp_output: *mut c_void, len: usize) -> u32 {
    let size = size_of::<T>() as u32;
    debug!("info_write_array({:#?}, {:#?}, {}) -> {}", data, lp_output, len, size);
    if lp_output == std::ptr::null_mut() {
        return size;
    }
    unsafe { std::ptr::copy(data, lp_output as *mut _, len); }
    size
}


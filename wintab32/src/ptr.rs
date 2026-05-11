/// [std::ptr::copy] that returns the amount of written bytes.
pub const unsafe fn copy<T>(src: *const T, dst: *mut T, count: usize) -> usize {
    let t_size = size_of::<T>();
    let size = t_size * count;
    unsafe {
        std::ptr::copy(src, dst, count);
    }
    size
}

/// [std::ptr::copy] that allows for unaligned pointers and returns the amount of written bytes.
/// This is required to write WinTab Packets, because, depending on the data requested,
/// amount of bytes might end up unaligned to 4 (default alignment).
/// E.g. when requesting full packets to be written, Packet size is 76, and the application
/// using WinTab expects packets to be written in succession, without any padding.
pub const unsafe fn copy_unaligned<T>(src: *const T, dst: *mut T, count: usize) -> usize {
    let t_size = size_of::<T>();
    let size = t_size * count;
    unsafe {
        std::ptr::write_unaligned(dst, std::ptr::read_unaligned(src));
    }
    size
}


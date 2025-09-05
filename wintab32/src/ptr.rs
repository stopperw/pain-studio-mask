pub const unsafe fn copy<T>(src: *const T, dst: *mut T, count: usize) -> usize {
    let t_size = size_of::<T>();
    let size = t_size * count;
    unsafe {
        std::ptr::copy(src, dst, count);
    }
    size
}

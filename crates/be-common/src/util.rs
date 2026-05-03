pub fn aligned_size(size: usize, alignment: usize) -> usize {
    (size + alignment - 1) / alignment * alignment
}

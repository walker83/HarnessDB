pub fn aligned_size(size: usize, alignment: usize) -> usize {
    size.div_ceil(alignment) * alignment
}

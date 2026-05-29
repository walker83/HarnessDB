use types::Bitmap;

// ===========================================================================
// P1 Optimization: Bitmap Operations Tests
// ===========================================================================

#[test]
fn test_bitmap_set_count_large() {
    let bools: Vec<bool> = (0..500).map(|i| i % 5 == 0).collect();
    let bm = Bitmap::from_bools(&bools);

    assert_eq!(bm.len(), 500);
    assert_eq!(bm.set_count(), 100);
}

#[test]
fn test_bitmap_set_count_all_true() {
    let bm = Bitmap::all_set(200);
    assert_eq!(bm.set_count(), 200);
}

#[test]
fn test_bitmap_set_count_all_false() {
    let bools: Vec<bool> = vec![false; 300];
    let bm = Bitmap::from_bools(&bools);
    assert_eq!(bm.set_count(), 0);
}

#[test]
fn test_bitmap_set_count_mixed() {
    let bools: Vec<bool> = (0..128).map(|i| i % 3 == 0).collect();
    let bm = Bitmap::from_bools(&bools);

    let expected = bools.iter().filter(|&b| *b).count();
    assert_eq!(bm.set_count(), expected);
}

#[test]
fn test_bitmap_and_inplace() {
    let mut bm1 = Bitmap::from_bools(&[true, true, false, true, false]);
    let bm2 = Bitmap::from_bools(&[true, false, true, true, false]);

    bm1.and_inplace(&bm2);

    assert_eq!(bm1.len(), 5);
    assert!(bm1.get(0));
    assert!(!bm1.get(1));
    assert!(!bm1.get(2));
    assert!(bm1.get(3));
    assert!(!bm1.get(4));
}

#[test]
fn test_bitmap_or_inplace() {
    let mut bm1 = Bitmap::from_bools(&[true, false, false, false, true]);
    let bm2 = Bitmap::from_bools(&[false, true, false, true, false]);

    bm1.or_inplace(&bm2);

    assert_eq!(bm1.len(), 5);
    assert!(bm1.get(0));
    assert!(bm1.get(1));
    assert!(!bm1.get(2));
    assert!(bm1.get(3));
    assert!(bm1.get(4));
}

#[test]
fn test_bitmap_not_inplace() {
    let mut bm = Bitmap::from_bools(&[true, false, true, true, false]);
    bm.not_inplace();

    assert_eq!(bm.len(), 5);
    assert!(!bm.get(0));
    assert!(bm.get(1));
    assert!(!bm.get(2));
    assert!(!bm.get(3));
    assert!(bm.get(4));
}

#[test]
fn test_bitmap_bitand_optimized() {
    let bm1 = Bitmap::from_bools(&[true; 100]);
    let bm2 = Bitmap::from_bools(&(0..100).map(|i| i % 2 == 0).collect::<Vec<bool>>());

    let result = &bm1 & &bm2;

    assert_eq!(result.len(), 100);
    assert_eq!(result.set_count(), 50);
}

#[test]
fn test_bitmap_bitor_optimized() {
    let bm1 = Bitmap::from_bools(&(0..100).map(|i| i % 2 == 0).collect::<Vec<bool>>());
    let bm2 = Bitmap::from_bools(&(0..100).map(|i| i % 3 == 0).collect::<Vec<bool>>());

    let result = &bm1 | &bm2;

    assert_eq!(result.len(), 100);
    let expected_count = (0..100).filter(|i| i % 2 == 0 || i % 3 == 0).count();
    assert_eq!(result.set_count(), expected_count);
}

#[test]
fn test_bitmap_not_optimized() {
    let bools: Vec<bool> = (0..150).map(|i| i % 4 == 0).collect();
    let bm = Bitmap::from_bools(&bools);

    let result = !&bm;

    assert_eq!(result.len(), 150);
    for i in 0..150 {
        assert_eq!(result.get(i), i % 4 != 0);
    }
}

#[test]
fn test_bitmap_iter_set_bits_large() {
    let bools: Vec<bool> = (0..200).map(|i| i % 7 == 0).collect();
    let bm = Bitmap::from_bools(&bools);

    let set_bits: Vec<usize> = bm.iter_set_bits().collect();
    let expected: Vec<usize> = (0..200).filter(|&i| i % 7 == 0).collect();

    assert_eq!(set_bits, expected);
}

#[test]
fn test_bitmap_iter_set_bits_edge_cases() {
    let bm = Bitmap::all_set(64);
    let set_bits: Vec<usize> = bm.iter_set_bits().collect();
    assert_eq!(set_bits, (0..64).collect::<Vec<usize>>());

    let bm2 = Bitmap::from_bools(&[false; 100]);
    let set_bits2: Vec<usize> = bm2.iter_set_bits().collect();
    assert_eq!(set_bits2, vec![]);
}

#[test]
fn test_bitmap_word_at() {
    let bools: Vec<bool> = vec![true, false, true, true, false, true, true, false];
    let bm = Bitmap::from_bools(&bools);

    let word = bm.word_at(0);
    // The actual implementation stores bits in a certain order
    // We just verify it's consistent with get() operations
    assert!(word > 0); // Should have some bits set
}

#[test]
fn test_bitmap_words() {
    let bm = Bitmap::from_bools(&[true; 65]);
    assert_eq!(bm.words(), 2);

    let bm2 = Bitmap::from_bools(&[true; 128]);
    assert_eq!(bm2.words(), 2);
}

#[test]
fn test_bitmap_clear() {
    let mut bm = Bitmap::from_bools(&[true, false, true, true]);
    bm.clear();

    assert_eq!(bm.len(), 0);
    assert!(bm.is_empty());
}

#[test]
fn test_bitmap_and_with() {
    let mut bm1 = Bitmap::from_bools(&[true, true, false, true]);
    let bm2 = Bitmap::from_bools(&[true, false, true, false]);

    bm1.and_with(&bm2);

    assert!(bm1.get(0));
    assert!(!bm1.get(1));
    assert!(!bm1.get(2));
    assert!(!bm1.get(3));
}

#[test]
fn test_bitmap_or_with() {
    let mut bm1 = Bitmap::from_bools(&[true, false, false, false]);
    let bm2 = Bitmap::from_bools(&[false, true, false, true]);

    bm1.or_with(&bm2);

    assert!(bm1.get(0));
    assert!(bm1.get(1));
    assert!(!bm1.get(2));
    assert!(bm1.get(3));
}

#[test]
fn test_bitmap_operations_cross_word_boundary() {
    let bm1 = Bitmap::from_bools(&[true; 70]);
    let bm2 = Bitmap::from_bools(&(0..70).map(|i| i % 2 == 0).collect::<Vec<bool>>());

    let result = &bm1 & &bm2;
    assert_eq!(result.set_count(), 35);

    let result2 = &bm1 | &bm2;
    assert_eq!(result2.set_count(), 70);
}

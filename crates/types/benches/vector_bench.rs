use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use types::{
    Bitmap, Block, DataType, Field, Float64Vector, Int64Vector, Schema,
    StringVector, Vector,
};

// ---------------------------------------------------------------------------
// Int64Vector creation from 1M elements
// ---------------------------------------------------------------------------

fn bench_int64_creation(c: &mut Criterion) {
    let data: Vec<i64> = (0..1_000_000).collect();

    c.bench_function("int64_vector_creation_1m", |b| {
        b.iter(|| {
            let v = Int64Vector::from_vec(black_box(data.clone()));
            black_box(&v);
        });
    });
}

// ---------------------------------------------------------------------------
// Int64Vector creation from nullable Vec<Option<i64>> 1M elements
// ---------------------------------------------------------------------------

fn bench_int64_nullable_creation(c: &mut Criterion) {
    let data: Vec<Option<i64>> = (0..1_000_000)
        .map(|i| if i % 10 == 0 { None } else { Some(i) })
        .collect();

    c.bench_function("int64_vector_nullable_creation_1m", |b| {
        b.iter(|| {
            let v = Int64Vector::from_nullable_vec(black_box(data.clone()));
            black_box(&v);
        });
    });
}

// ---------------------------------------------------------------------------
// Vector filter on 1M elements (50% selectivity)
// ---------------------------------------------------------------------------

fn bench_vector_filter(c: &mut Criterion) {
    let size = 1_000_000;
    let data: Vec<i64> = (0..size).collect();
    let vector = Int64Vector::from_vec(data);

    // Build selection bitmap with 50% selectivity
    let mut selection = Bitmap::with_capacity(size);
    for i in 0..size {
        selection.push(i % 2 == 0);
    }

    c.bench_function("int64_vector_filter_1m_50pct", |b| {
        b.iter(|| {
            let filtered = black_box(&vector).filter(black_box(&selection));
            black_box(&filtered);
        });
    });
}

// ---------------------------------------------------------------------------
// Vector slice on 1M elements
// ---------------------------------------------------------------------------

fn bench_vector_slice(c: &mut Criterion) {
    let size = 1_000_000;
    let data: Vec<i64> = (0..size).collect();
    let vector = Int64Vector::from_vec(data);

    c.bench_function("int64_vector_slice_1m", |b| {
        b.iter(|| {
            let sliced = black_box(&vector).slice(black_box(1000), black_box(100_000));
            black_box(&sliced);
        });
    });
}

// ---------------------------------------------------------------------------
// Bitmap intersection on 1M bits
// ---------------------------------------------------------------------------

fn bench_bitmap_intersection(c: &mut Criterion) {
    let size = 1_000_000;

    let mut bm1 = Bitmap::with_capacity(size);
    let mut bm2 = Bitmap::with_capacity(size);
    for i in 0..size {
        bm1.push(i % 3 == 0);
        bm2.push(i % 5 == 0);
    }

    c.bench_function("bitmap_intersection_1m", |b| {
        b.iter(|| {
            let result = black_box(&bm1) & black_box(&bm2);
            black_box(&result);
        });
    });
}

// ---------------------------------------------------------------------------
// Bitmap union on 1M bits
// ---------------------------------------------------------------------------

fn bench_bitmap_union(c: &mut Criterion) {
    let size = 1_000_000;

    let mut bm1 = Bitmap::with_capacity(size);
    let mut bm2 = Bitmap::with_capacity(size);
    for i in 0..size {
        bm1.push(i % 3 == 0);
        bm2.push(i % 5 == 0);
    }

    c.bench_function("bitmap_union_1m", |b| {
        b.iter(|| {
            let result = black_box(&bm1) | black_box(&bm2);
            black_box(&result);
        });
    });
}

// ---------------------------------------------------------------------------
// Bitmap NOT on 1M bits
// ---------------------------------------------------------------------------

fn bench_bitmap_not(c: &mut Criterion) {
    let size = 1_000_000;
    let mut bm = Bitmap::with_capacity(size);
    for i in 0..size {
        bm.push(i % 2 == 0);
    }

    c.bench_function("bitmap_not_1m", |b| {
        b.iter(|| {
            let result = !black_box(&bm);
            black_box(&result);
        });
    });
}

// ---------------------------------------------------------------------------
// Block creation with 10 columns x 1M rows
// ---------------------------------------------------------------------------

fn bench_block_creation(c: &mut Criterion) {
    let num_rows = 1_000_000;
    let num_cols = 10;

    let fields: Vec<Field> = (0..num_cols)
        .map(|i| {
            let name = format!("col_{}", i);
            if i < 5 {
                Field::new(name, DataType::Int64, false)
            } else if i < 8 {
                Field::new(name, DataType::Float64, false)
            } else {
                Field::new(name, DataType::String, false)
            }
        })
        .collect();
    let schema = Schema::new(fields);

    let int_data: Vec<i64> = (0..num_rows).map(|i| i as i64).collect();
    let float_data: Vec<f64> = (0..num_rows).map(|i| i as f64 * 1.5).collect();
    let string_data: Vec<&str> = (0..num_rows).map(|_| "benchmark_value").collect();

    c.bench_function("block_creation_10cols_1m", |b| {
        b.iter(|| {
            let mut columns = Vec::with_capacity(num_cols);
            for i in 0..num_cols {
                if i < 5 {
                    columns.push(Vector::Int64(Int64Vector::from_vec(black_box(int_data.clone()))));
                } else if i < 8 {
                    columns.push(Vector::Float64(Float64Vector::from_vec(black_box(float_data.clone()))));
                } else {
                    columns.push(Vector::String(StringVector::from_vec(black_box(string_data.clone()))));
                }
            }
            let block = Block::new(black_box(schema.clone()), columns);
            black_box(&block);
        });
    });
}

// ---------------------------------------------------------------------------
// Block filter with selection bitmap
// ---------------------------------------------------------------------------

fn bench_block_filter(c: &mut Criterion) {
    let num_rows = 1_000_000;

    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("value", DataType::Float64, false),
        Field::new("name", DataType::String, false),
    ]);

    let columns = vec![
        Vector::Int64(Int64Vector::from_vec((0..num_rows).collect())),
        Vector::Float64(Float64Vector::from_vec((0..num_rows).map(|i| i as f64).collect())),
        Vector::String(StringVector::from_vec(
            (0..num_rows).map(|_| "value").collect(),
        )),
    ];

    let block = Block::new(schema, columns);

    // Build selection bitmap with ~50% selectivity
    let mut selection = Bitmap::with_capacity(num_rows);
    for i in 0..num_rows {
        selection.push(i % 2 == 0);
    }

    c.bench_function("block_filter_3cols_1m_50pct", |b| {
        b.iter(|| {
            let filtered = black_box(&block).filter(black_box(&selection));
            black_box(&filtered);
        });
    });
}

// ---------------------------------------------------------------------------
// Scalar row-by-row vs vectorized batch processing comparison
// ---------------------------------------------------------------------------

fn bench_scalar_vs_vectorized(c: &mut Criterion) {
    let size = 1_000_000;
    let data: Vec<i64> = (0..size).map(|i| i as i64).collect();
    let vector = Int64Vector::from_vec(data.clone());

    let mut group = c.benchmark_group("scalar_vs_vectorized_sum");

    // Scalar: row-by-row iteration
    group.bench_function("scalar_row_by_row_sum", |b| {
        b.iter(|| {
            let mut sum: i64 = 0;
            for i in 0..black_box(&vector).len() {
                if let Some(v) = black_box(&vector).get(i) {
                    sum += v;
                }
            }
            black_box(sum);
        });
    });

    // Vectorized: direct data access
    group.bench_function("vectorized_batch_sum", |b| {
        b.iter(|| {
            let data = black_box(&vector).data();
            let sum: i64 = data.iter().sum();
            black_box(sum);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Scalar vs vectorized filter comparison
// ---------------------------------------------------------------------------

fn bench_scalar_vs_vectorized_filter(c: &mut Criterion) {
    let size = 1_000_000;
    let data: Vec<i64> = (0..size).collect();
    let vector = Int64Vector::from_vec(data);

    let mut group = c.benchmark_group("scalar_vs_vectorized_filter");

    let threshold = size as i64 / 2;

    // Scalar approach: check each element individually
    group.bench_function("scalar_filter_loop", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for i in 0..black_box(&vector).len() {
                if let Some(v) = black_box(&vector).get(i) {
                    if v > threshold {
                        results.push(v);
                    }
                }
            }
            black_box(&results);
        });
    });

    // Vectorized approach: build bitmap then use filter
    group.bench_function("vectorized_bitmap_filter", |b| {
        b.iter(|| {
            let data = black_box(&vector).data();
            let mut sel = Bitmap::with_capacity(data.len());
            for &v in data {
                sel.push(v > threshold);
            }
            let filtered = black_box(&vector).filter(&sel);
            black_box(&filtered);
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// StringVector operations
// ---------------------------------------------------------------------------

fn bench_string_vector_creation(c: &mut Criterion) {
    let data: Vec<&str> = (0..100_000).map(|i| {
        if i % 100 == 0 { "short" } else { "a_medium_length_string_value" }
    }).collect();

    c.bench_function("string_vector_creation_100k", |b| {
        b.iter(|| {
            let v = StringVector::from_vec(black_box(data.clone()));
            black_box(&v);
        });
    });
}

fn bench_string_vector_filter(c: &mut Criterion) {
    let size = 100_000;
    let data: Vec<&str> = (0..size).map(|_| "hello_world").collect();
    let vector = StringVector::from_vec(data);

    let mut selection = Bitmap::with_capacity(size);
    for i in 0..size {
        selection.push(i % 2 == 0);
    }

    c.bench_function("string_vector_filter_100k_50pct", |b| {
        b.iter(|| {
            let filtered = black_box(&vector).filter(black_box(&selection));
            black_box(&filtered);
        });
    });
}

// ---------------------------------------------------------------------------
// Bitmap creation from bools at different sizes
// ---------------------------------------------------------------------------

fn bench_bitmap_from_bools_parametric(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitmap_from_bools");
    for &size in &[1_000, 10_000, 100_000, 1_000_000] {
        let bools: Vec<bool> = (0..size).map(|i| i % 2 == 0).collect();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &bools, |b, bools| {
            b.iter(|| {
                let bm = Bitmap::from_bools(black_box(bools));
                black_box(&bm);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_int64_creation,
    bench_int64_nullable_creation,
    bench_vector_filter,
    bench_vector_slice,
    bench_bitmap_intersection,
    bench_bitmap_union,
    bench_bitmap_not,
    bench_block_creation,
    bench_block_filter,
    bench_scalar_vs_vectorized,
    bench_scalar_vs_vectorized_filter,
    bench_string_vector_creation,
    bench_string_vector_filter,
    bench_bitmap_from_bools_parametric,
);

criterion_main!(benches);

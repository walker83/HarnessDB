use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tpch_bench::{data_gen::TpchData, queries, TpchBenchmark};

// ---------------------------------------------------------------------------
// Data generation benchmarks
// ---------------------------------------------------------------------------

fn bench_data_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tpch_data_generation");

    group.bench_function("generate_sf001", |b| {
        b.iter(|| {
            let data = TpchData::generate_sf001();
            black_box(data.lineitem.num_rows());
        });
    });

    group.bench_function("generate_tiny", |b| {
        b.iter(|| {
            let data = TpchData::generate_tiny();
            black_box(data.lineitem.num_rows());
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Individual TPC-H query planning benchmarks
// ---------------------------------------------------------------------------

fn bench_tpch_queries(c: &mut Criterion) {
    let bench = TpchBenchmark::new_tiny();

    let mut group = c.benchmark_group("tpch_query_planning");
    for (name, sql) in queries::all_queries() {
        group.bench_with_input(
            BenchmarkId::new("plan", name),
            &(name, sql),
            |b, &(qname, qsql)| {
                b.iter(|| {
                    let result = black_box(&bench).run_sql(qname, qsql);
                    black_box(&result);
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Block operation benchmarks on TPC-H data
// ---------------------------------------------------------------------------

fn bench_lineitem_filter(c: &mut Criterion) {
    let data = TpchData::generate_sf001();
    let lineitem = &data.lineitem;

    // Filter on returnflag = 'R'
    let returnflag_col = lineitem.column_by_name("l_returnflag").unwrap().1;
    let num_rows = lineitem.num_rows();

    let mut selection = types::Bitmap::with_capacity(num_rows);
    for i in 0..num_rows {
        let val = returnflag_col.scalar_at(i);
        let is_r = matches!(val, types::ScalarValue::String(s) if s == "R");
        selection.push(is_r);
    }

    c.bench_function("tpch_lineitem_filter_returnflag_R", |b| {
        b.iter(|| {
            let filtered = black_box(lineitem).filter(black_box(&selection));
            black_box(&filtered);
        });
    });
}

fn bench_lineitem_projection(c: &mut Criterion) {
    let data = TpchData::generate_sf001();
    let lineitem = &data.lineitem;

    // Project down to a few columns (simulating Q6: shipdate, discount, quantity, extendedprice)
    let indices: Vec<usize> = lineitem
        .schema()
        .fields()
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(
            f.name.as_str(),
            "l_shipdate" | "l_discount" | "l_quantity" | "l_extendedprice"
        ))
        .map(|(i, _)| i)
        .collect();

    c.bench_function("tpch_lineitem_projection_4cols", |b| {
        b.iter(|| {
            let projected = black_box(lineitem).project(black_box(&indices));
            black_box(&projected);
        });
    });
}

fn bench_orders_slice(c: &mut Criterion) {
    let data = TpchData::generate_sf001();
    let orders = &data.orders;

    c.bench_function("tpch_orders_slice_1000", |b| {
        b.iter(|| {
            let sliced = black_box(orders).slice(black_box(100), black_box(1000));
            black_box(&sliced);
        });
    });
}

// ---------------------------------------------------------------------------
// Catalog setup benchmark
// ---------------------------------------------------------------------------

fn bench_catalog_setup(c: &mut Criterion) {
    c.bench_function("tpch_catalog_setup", |b| {
        b.iter(|| {
            let bench = TpchBenchmark::new_tiny();
            black_box(&bench);
        });
    });
}

// ---------------------------------------------------------------------------
// Full pipeline benchmark: generate + plan Q6 (simplest query)
// ---------------------------------------------------------------------------

fn bench_full_pipeline_q6(c: &mut Criterion) {
    c.bench_function("tpch_full_pipeline_q6", |b| {
        b.iter(|| {
            let bench = TpchBenchmark::new_tiny();
            let result = bench.run_query(6);
            black_box(&result);
        });
    });
}

criterion_group!(
    tpch_benches,
    bench_data_generation,
    bench_tpch_queries,
    bench_lineitem_filter,
    bench_lineitem_projection,
    bench_orders_slice,
    bench_catalog_setup,
    bench_full_pipeline_q6,
);

criterion_main!(tpch_benches);

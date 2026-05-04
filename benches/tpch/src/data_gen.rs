use types::{
    vector::{Float64Vector, Int32Vector, Int64Vector, StringVector},
    Block, DataType, Field, Schema, Vector,
};

/// TPC-H scale factor 0.01 data generator.
/// Generates all 8 TPC-H tables as Block objects with realistic data.
// ---------------------------------------------------------------------------
// Nation table
// ---------------------------------------------------------------------------
/// Schema: n_nationkey (PK), n_name, n_regionkey (FK), n_comment
pub fn nation_schema() -> Schema {
    Schema::new(vec![
        Field::new("n_nationkey", DataType::Int64, false),
        Field::new("n_name", DataType::String, false),
        Field::new("n_regionkey", DataType::Int64, false),
        Field::new("n_comment", DataType::String, true),
    ])
}

pub fn generate_nation() -> Block {
    let nations: Vec<(i64, &str, i64, &str)> = vec![
        (0, "ALGERIA", 0, "hareful final packages detect slyly. carefully"),
        (1, "ARGENTINA", 1, "al foxes promise slyly"),
        (2, "BRAZIL", 1, "y alongside of the pending deposits"),
        (3, "CANADA", 1, "eas hang ironic, silent packages"),
        (4, "EGYPT", 4, "y above the carefully unusual theodolites"),
        (5, "ETHIOPIA", 0, "ven packages wake quickly"),
        (6, "FRANCE", 3, "refully final requests"),
        (7, "GERMANY", 3, "l platelets use slyly"),
        (8, "INDIA", 2, "ss excuses cajole slyly"),
        (9, "INDONESIA", 2, "slyly express requests"),
        (10, "IRAN", 4, "efully special requests"),
        (11, "IRAQ", 4, "nal foxes sleep furiously"),
        (12, "JAPAN", 2, "ly pending requests"),
        (13, "JORDAN", 4, "ously special deposits wake slyly"),
        (14, "KENYA", 0, "ng, unusual platelets wake"),
    ];

    let schema = nation_schema();
    let mut keys = Vec::new();
    let mut names = Vec::new();
    let mut region_keys = Vec::new();
    let mut comments = Vec::new();

    for (nk, nm, rk, cm) in &nations {
        keys.push(Some(*nk));
        names.push(Some(*nm));
        region_keys.push(Some(*rk));
        comments.push(Some(*cm));
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(keys)),
            Vector::String(StringVector::from_option_vec(
                names.into_iter().map(|o| o.map(|s| s.to_string())).collect(),
            )),
            Vector::Int64(Int64Vector::from_nullable_vec(region_keys)),
            Vector::String(StringVector::from_option_vec(
                comments.into_iter().map(|o| o.map(|s| s.to_string())).collect(),
            )),
        ],
    )
}

// ---------------------------------------------------------------------------
// Region table
// ---------------------------------------------------------------------------

pub fn region_schema() -> Schema {
    Schema::new(vec![
        Field::new("r_regionkey", DataType::Int64, false),
        Field::new("r_name", DataType::String, false),
        Field::new("r_comment", DataType::String, true),
    ])
}

pub fn generate_region() -> Block {
    let regions: Vec<(i64, &str, &str)> = vec![
        (0, "AFRICA", "lar deposits"),
        (1, "AMERICA", "hs use ironic, even requests"),
        (2, "ASIA", "ges. thinly even pinto beans are"),
        (3, "EUROPE", "ly final courts cajole"),
        (4, "MIDDLE EAST", "uickly special accounts"),
    ];

    let schema = region_schema();
    let mut keys = Vec::new();
    let mut names = Vec::new();
    let mut comments = Vec::new();

    for (rk, rn, rc) in &regions {
        keys.push(Some(*rk));
        names.push(Some(*rn));
        comments.push(Some(*rc));
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(keys)),
            Vector::String(StringVector::from_option_vec(
                names.into_iter().map(|o| o.map(|s| s.to_string())).collect(),
            )),
            Vector::String(StringVector::from_option_vec(
                comments.into_iter().map(|o| o.map(|s| s.to_string())).collect(),
            )),
        ],
    )
}

// ---------------------------------------------------------------------------
// Supplier table
// ---------------------------------------------------------------------------

pub fn supplier_schema() -> Schema {
    Schema::new(vec![
        Field::new("s_suppkey", DataType::Int64, false),
        Field::new("s_name", DataType::String, false),
        Field::new("s_address", DataType::String, false),
        Field::new("s_nationkey", DataType::Int64, false),
        Field::new("s_phone", DataType::String, false),
        Field::new("s_acctbal", DataType::Float64, false),
        Field::new("s_comment", DataType::String, true),
    ])
}

pub fn generate_supplier(count: usize) -> Block {
    let schema = supplier_schema();
    let mut keys = Vec::new();
    let mut names = Vec::new();
    let mut addresses = Vec::new();
    let mut nation_keys = Vec::new();
    let mut phones = Vec::new();
    let mut acct_bals = Vec::new();
    let mut comments = Vec::new();

    for i in 0..count {
        keys.push(Some(i as i64));
        names.push(Some(format!("Supplier#{:09}", i)));
        addresses.push(Some(format!("{} Street", i)));
        nation_keys.push(Some((i % 15) as i64));
        phones.push(Some(format!("{:03}-{:03}-{:04}", 10 + i % 20, 100 + i % 900, i % 10000)));
        acct_bals.push(Some(-1000.0 + (i as f64 * 99.7) % 10000.0));
        comments.push(Some(format!("Comment for supplier {}", i)));
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(keys)),
            Vector::String(StringVector::from_option_vec(names)),
            Vector::String(StringVector::from_option_vec(addresses)),
            Vector::Int64(Int64Vector::from_nullable_vec(nation_keys)),
            Vector::String(StringVector::from_option_vec(phones)),
            Vector::Float64(Float64Vector::from_nullable_vec(acct_bals)),
            Vector::String(StringVector::from_option_vec(comments)),
        ],
    )
}

// ---------------------------------------------------------------------------
// Part table
// ---------------------------------------------------------------------------

pub fn part_schema() -> Schema {
    Schema::new(vec![
        Field::new("p_partkey", DataType::Int64, false),
        Field::new("p_name", DataType::String, false),
        Field::new("p_mfgr", DataType::String, false),
        Field::new("p_brand", DataType::String, false),
        Field::new("p_type", DataType::String, false),
        Field::new("p_size", DataType::Int32, false),
        Field::new("p_container", DataType::String, false),
        Field::new("p_retailprice", DataType::Float64, false),
        Field::new("p_comment", DataType::String, true),
    ])
}

pub fn generate_part(count: usize) -> Block {
    let schema = part_schema();
    let manufacturers = ["Manufacturer#1", "Manufacturer#2", "Manufacturer#3", "Manufacturer#4", "Manufacturer#5"];
    let brands = ["Brand#11", "Brand#12", "Brand#13", "Brand#14", "Brand#15",
                  "Brand#21", "Brand#22", "Brand#23", "Brand#24", "Brand#25",
                  "Brand#31", "Brand#32", "Brand#33", "Brand#34", "Brand#35",
                  "Brand#41", "Brand#42", "Brand#43", "Brand#44", "Brand#45"];
    let types = ["Standard", "Small", "Medium", "Large", "Economy"];
    let materials = ["Tin", "Nickel", "Brass", "Steel", "Copper", "Aluminum", "Wood", "Pine"];
    let containers = ["SM CASE", "SM BOX", "SM PACK", "SM PKG",
                      "MED BAG", "MED BOX", "MED PKG", "MED PACK",
                      "LG CASE", "LG BOX", "LG PACK", "LG PKG"];

    let mut keys = Vec::new();
    let mut names = Vec::new();
    let mut mfgrs = Vec::new();
    let mut brand_list = Vec::new();
    let mut ptypes = Vec::new();
    let mut sizes = Vec::new();
    let mut container_list = Vec::new();
    let mut prices = Vec::new();
    let mut comments = Vec::new();

    for i in 0..count {
        keys.push(Some(i as i64));
        let material = materials[i % materials.len()];
        let ptype = types[i % types.len()];
        names.push(Some(format!("{} {} Part#{}", ptype, material, i)));
        mfgrs.push(Some(manufacturers[i % manufacturers.len()].to_string()));
        brand_list.push(Some(brands[i % brands.len()].to_string()));
        ptypes.push(Some(format!("{} ANODIZED {}", ptype, material)));
        sizes.push(Some(((i % 50) + 1) as i32));
        container_list.push(Some(containers[i % containers.len()].to_string()));
        prices.push(Some(900.0 + (i as f64 * 1.27) % 1800.0));
        comments.push(Some(format!("Part comment {}", i)));
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(keys)),
            Vector::String(StringVector::from_option_vec(names)),
            Vector::String(StringVector::from_option_vec(mfgrs)),
            Vector::String(StringVector::from_option_vec(brand_list)),
            Vector::String(StringVector::from_option_vec(ptypes)),
            Vector::Int32(Int32Vector::from_nullable_vec(sizes)),
            Vector::String(StringVector::from_option_vec(container_list)),
            Vector::Float64(Float64Vector::from_nullable_vec(prices)),
            Vector::String(StringVector::from_option_vec(comments)),
        ],
    )
}

// ---------------------------------------------------------------------------
// PartSupp table
// ---------------------------------------------------------------------------

pub fn partsupp_schema() -> Schema {
    Schema::new(vec![
        Field::new("ps_partkey", DataType::Int64, false),
        Field::new("ps_suppkey", DataType::Int64, false),
        Field::new("ps_availqty", DataType::Int32, false),
        Field::new("ps_supplycost", DataType::Float64, false),
        Field::new("ps_comment", DataType::String, true),
    ])
}

pub fn generate_partsupp(part_count: usize, supplier_count: usize) -> Block {
    let schema = partsupp_schema();
    let rows_per_part = 4; // TPC-H spec: each part has 4 suppliers

    let mut part_keys = Vec::new();
    let mut supp_keys = Vec::new();
    let mut avail_qtys = Vec::new();
    let mut supply_costs = Vec::new();
    let mut comments = Vec::new();

    for p in 0..part_count {
        for s in 0..rows_per_part {
            let supp_key = ((p + s) % supplier_count) as i64;
            part_keys.push(Some(p as i64));
            supp_keys.push(Some(supp_key));
            avail_qtys.push(Some((100 + (p * rows_per_part + s) % 9000) as i32));
            supply_costs.push(Some(1.0 + ((p * 4 + s) as f64 * 5.17) % 1000.0));
            comments.push(Some(format!("Partsupp comment for part {} supplier {}", p, supp_key)));
        }
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(part_keys)),
            Vector::Int64(Int64Vector::from_nullable_vec(supp_keys)),
            Vector::Int32(Int32Vector::from_nullable_vec(avail_qtys)),
            Vector::Float64(Float64Vector::from_nullable_vec(supply_costs)),
            Vector::String(StringVector::from_option_vec(comments)),
        ],
    )
}

// ---------------------------------------------------------------------------
// Customer table
// ---------------------------------------------------------------------------

pub fn customer_schema() -> Schema {
    Schema::new(vec![
        Field::new("c_custkey", DataType::Int64, false),
        Field::new("c_name", DataType::String, false),
        Field::new("c_address", DataType::String, false),
        Field::new("c_nationkey", DataType::Int64, false),
        Field::new("c_phone", DataType::String, false),
        Field::new("c_acctbal", DataType::Float64, false),
        Field::new("c_mktsegment", DataType::String, false),
        Field::new("c_comment", DataType::String, true),
    ])
}

pub fn generate_customer(count: usize) -> Block {
    let schema = customer_schema();
    let segments = ["AUTOMOBILE", "BUILDING", "FURNITURE", "MACHINERY", "HOUSEHOLD"];

    let mut keys = Vec::new();
    let mut names = Vec::new();
    let mut addresses = Vec::new();
    let mut nation_keys = Vec::new();
    let mut phones = Vec::new();
    let mut acct_bals = Vec::new();
    let mut segments_list = Vec::new();
    let mut comments = Vec::new();

    for i in 0..count {
        keys.push(Some(i as i64));
        names.push(Some(format!("Customer#{:09}", i)));
        addresses.push(Some(format!("{} Main St", i)));
        nation_keys.push(Some((i % 15) as i64));
        phones.push(Some(format!("{:02}-{:03}-{:04}", 10 + i % 30, 100 + i % 900, i % 10000)));
        acct_bals.push(Some(-500.0 + (i as f64 * 73.3) % 12000.0));
        segments_list.push(Some(segments[i % segments.len()].to_string()));
        comments.push(Some(format!("Customer comment {}", i)));
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(keys)),
            Vector::String(StringVector::from_option_vec(names)),
            Vector::String(StringVector::from_option_vec(addresses)),
            Vector::Int64(Int64Vector::from_nullable_vec(nation_keys)),
            Vector::String(StringVector::from_option_vec(phones)),
            Vector::Float64(Float64Vector::from_nullable_vec(acct_bals)),
            Vector::String(StringVector::from_option_vec(segments_list)),
            Vector::String(StringVector::from_option_vec(comments)),
        ],
    )
}

// ---------------------------------------------------------------------------
// Orders table
// ---------------------------------------------------------------------------

pub fn orders_schema() -> Schema {
    Schema::new(vec![
        Field::new("o_orderkey", DataType::Int64, false),
        Field::new("o_custkey", DataType::Int64, false),
        Field::new("o_orderstatus", DataType::String, false),
        Field::new("o_totalprice", DataType::Float64, false),
        Field::new("o_orderdate", DataType::Int32, false),
        Field::new("o_orderpriority", DataType::String, false),
        Field::new("o_clerk", DataType::String, false),
        Field::new("o_shippriority", DataType::Int32, false),
        Field::new("o_comment", DataType::String, true),
    ])
}

pub fn generate_orders(count: usize, customer_count: usize) -> Block {
    let schema = orders_schema();
    let statuses = ["O", "F", "P"];
    let priorities = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"];

    let mut order_keys = Vec::new();
    let mut cust_keys = Vec::new();
    let mut status_list = Vec::new();
    let mut total_prices = Vec::new();
    let mut order_dates = Vec::new();
    let mut priority_list = Vec::new();
    let mut clerks = Vec::new();
    let mut ship_priorities = Vec::new();
    let mut comments = Vec::new();

    // Date range: 1992-01-01 to 1998-12-31
    // Represented as days since epoch: 1992-01-01 = 7305, 1998-12-31 = 10564
    let date_start = 7305_i32;
    let date_range = 3260_i32; // ~9 years

    for i in 0..count {
        order_keys.push(Some(i as i64));
        cust_keys.push(Some((i % customer_count) as i64));
        status_list.push(Some(statuses[i % statuses.len()].to_string()));
        total_prices.push(Some(100.0 + (i as f64 * 37.3) % 500000.0));
        order_dates.push(Some(date_start + (i as i32 % date_range)));
        priority_list.push(Some(priorities[i % priorities.len()].to_string()));
        clerks.push(Some(format!("Clerk#{:06}", i % 100)));
        ship_priorities.push(Some(0));
        comments.push(Some(format!("Order comment {}", i)));
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(order_keys)),
            Vector::Int64(Int64Vector::from_nullable_vec(cust_keys)),
            Vector::String(StringVector::from_option_vec(status_list)),
            Vector::Float64(Float64Vector::from_nullable_vec(total_prices)),
            Vector::Int32(Int32Vector::from_nullable_vec(order_dates)),
            Vector::String(StringVector::from_option_vec(priority_list)),
            Vector::String(StringVector::from_option_vec(clerks)),
            Vector::Int32(Int32Vector::from_nullable_vec(ship_priorities)),
            Vector::String(StringVector::from_option_vec(comments)),
        ],
    )
}

// ---------------------------------------------------------------------------
// Lineitem table
// ---------------------------------------------------------------------------

pub fn lineitem_schema() -> Schema {
    Schema::new(vec![
        Field::new("l_orderkey", DataType::Int64, false),
        Field::new("l_partkey", DataType::Int64, false),
        Field::new("l_suppkey", DataType::Int64, false),
        Field::new("l_linenumber", DataType::Int32, false),
        Field::new("l_quantity", DataType::Float64, false),
        Field::new("l_extendedprice", DataType::Float64, false),
        Field::new("l_discount", DataType::Float64, false),
        Field::new("l_tax", DataType::Float64, false),
        Field::new("l_returnflag", DataType::String, false),
        Field::new("l_linestatus", DataType::String, false),
        Field::new("l_shipdate", DataType::Int32, false),
        Field::new("l_commitdate", DataType::Int32, false),
        Field::new("l_receiptdate", DataType::Int32, false),
        Field::new("l_shipinstruct", DataType::String, false),
        Field::new("l_shipmode", DataType::String, false),
        Field::new("l_comment", DataType::String, true),
    ])
}

pub fn generate_lineitem(order_count: usize, part_count: usize, supplier_count: usize) -> Block {
    let schema = lineitem_schema();
    let return_flags = ["R", "A", "N"];
    let line_statuses = ["O", "F"];
    let ship_instructions = ["DELIVER IN PERSON", "COLLECT COD", "NONE", "TAKE BACK RETURN"];
    let ship_modes = ["REG AIR", "AIR", "RAIL", "TRUCK", "MAIL", "FOB", "SHIP"];

    let lines_per_order = 4; // TPC-H spec: 1-7 lines per order, we use 4

    let mut order_keys = Vec::new();
    let mut part_keys = Vec::new();
    let mut supp_keys = Vec::new();
    let mut line_numbers = Vec::new();
    let mut quantities = Vec::new();
    let mut extended_prices = Vec::new();
    let mut discounts = Vec::new();
    let mut taxes = Vec::new();
    let mut return_flag_list = Vec::new();
    let mut line_status_list = Vec::new();
    let mut ship_dates = Vec::new();
    let mut commit_dates = Vec::new();
    let mut receipt_dates = Vec::new();
    let mut ship_instructions_list = Vec::new();
    let mut ship_mode_list = Vec::new();
    let mut comments = Vec::new();

    // Date range similar to orders
    let date_start = 7305_i32;
    let date_range = 3260_i32;

    for o in 0..order_count {
        for l in 0..lines_per_order {
            let idx = o * lines_per_order + l;
            order_keys.push(Some(o as i64));
            part_keys.push(Some((o + l) as i64 % part_count as i64));
            supp_keys.push(Some((o + l) as i64 % supplier_count as i64));
            line_numbers.push(Some((l + 1) as i32));
            let qty = 1.0 + ((idx as f64 * 7.3) % 50.0);
            quantities.push(Some(qty));
            extended_prices.push(Some(qty * (900.0 + ((o + l) as f64 * 1.27) % 1800.0)));
            discounts.push(Some(((idx as f64 * 3.7) % 10.0) / 100.0));
            taxes.push(Some(((idx as f64 * 1.3) % 8.0) / 100.0));
            return_flag_list.push(Some(return_flags[idx % return_flags.len()].to_string()));
            line_status_list.push(Some(line_statuses[idx % line_statuses.len()].to_string()));

            let order_date = date_start + (o as i32 % date_range);
            ship_dates.push(Some(order_date + (l as i32 % 30)));
            commit_dates.push(Some(order_date + (l as i32 % 20)));
            receipt_dates.push(Some(order_date + (l as i32 % 35)));

            ship_instructions_list.push(Some(ship_instructions[idx % ship_instructions.len()].to_string()));
            ship_mode_list.push(Some(ship_modes[idx % ship_modes.len()].to_string()));
            comments.push(Some(format!("Lineitem comment {}", idx)));
        }
    }

    Block::new(
        schema,
        vec![
            Vector::Int64(Int64Vector::from_nullable_vec(order_keys)),
            Vector::Int64(Int64Vector::from_nullable_vec(part_keys)),
            Vector::Int64(Int64Vector::from_nullable_vec(supp_keys)),
            Vector::Int32(Int32Vector::from_nullable_vec(line_numbers)),
            Vector::Float64(Float64Vector::from_nullable_vec(quantities)),
            Vector::Float64(Float64Vector::from_nullable_vec(extended_prices)),
            Vector::Float64(Float64Vector::from_nullable_vec(discounts)),
            Vector::Float64(Float64Vector::from_nullable_vec(taxes)),
            Vector::String(StringVector::from_option_vec(return_flag_list)),
            Vector::String(StringVector::from_option_vec(line_status_list)),
            Vector::Int32(Int32Vector::from_nullable_vec(ship_dates)),
            Vector::Int32(Int32Vector::from_nullable_vec(commit_dates)),
            Vector::Int32(Int32Vector::from_nullable_vec(receipt_dates)),
            Vector::String(StringVector::from_option_vec(ship_instructions_list)),
            Vector::String(StringVector::from_option_vec(ship_mode_list)),
            Vector::String(StringVector::from_option_vec(comments)),
        ],
    )
}

// ---------------------------------------------------------------------------
// Generate all TPC-H tables at scale factor 0.01
// ---------------------------------------------------------------------------

/// Container for all TPC-H tables.
pub struct TpchData {
    pub nation: Block,
    pub region: Block,
    pub supplier: Block,
    pub part: Block,
    pub partsupp: Block,
    pub customer: Block,
    pub orders: Block,
    pub lineitem: Block,
}

impl TpchData {
    /// Generate all tables at scale factor 0.01 (small).
    ///
    /// Approximate row counts at SF 0.01:
    /// - nation: 15 (fixed)
    /// - region: 5 (fixed)
    /// - supplier: 100
    /// - part: 200
    /// - partsupp: 800 (4 per part)
    /// - customer: 150
    /// - orders: 1500
    /// - lineitem: 6000 (4 per order)
    pub fn generate_sf001() -> Self {
        let nation = generate_nation();
        let region = generate_region();
        let supplier = generate_supplier(100);
        let part = generate_part(200);
        let partsupp = generate_partsupp(200, 100);
        let customer = generate_customer(150);
        let orders = generate_orders(1500, 150);
        let lineitem = generate_lineitem(1500, 200, 100);

        Self {
            nation,
            region,
            supplier,
            part,
            partsupp,
            customer,
            orders,
            lineitem,
        }
    }

    /// Generate all tables at a smaller scale for quick benchmarks.
    pub fn generate_tiny() -> Self {
        let nation = generate_nation();
        let region = generate_region();
        let supplier = generate_supplier(10);
        let part = generate_part(20);
        let partsupp = generate_partsupp(20, 10);
        let customer = generate_customer(15);
        let orders = generate_orders(150, 15);
        let lineitem = generate_lineitem(150, 20, 10);

        Self {
            nation,
            region,
            supplier,
            part,
            partsupp,
            customer,
            orders,
            lineitem,
        }
    }
}

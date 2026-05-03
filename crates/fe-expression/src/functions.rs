use types::{ScalarValue, Vector, DataType};

pub struct FunctionRegistry;

impl FunctionRegistry {
    pub fn new() -> Self { Self }

    pub fn call(&self, name: &str, args: &[Vector]) -> Vector {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "abs" => self.abs(args),
            "ceil" | "ceiling" => self.ceil(args),
            "floor" => self.floor(args),
            "round" => self.round(args),
            "upper" => self.upper(args),
            "lower" => self.lower(args),
            "length" | "char_length" => self.length(args),
            "concat" => self.concat(args),
            "substring" | "substr" => self.substring(args),
            "trim" => self.trim(args),
            "coalesce" => self.coalesce(args),
            "ifnull" => self.ifnull(args),
            "nullif" => self.nullif(args),
            "cast" => args.first().cloned().unwrap_or_else(|| bool_vec(vec![])),
            "count" => int64_vec(vec![args.first().map(|v| v.len() as i64).unwrap_or(0)]),
            "sum" => self.sum(args),
            "avg" => self.avg(args),
            "min" => self.min(args),
            "max" => self.max(args),
            _ => {
                tracing::warn!("unknown function: {}", name);
                args.first().cloned().unwrap_or_else(|| bool_vec(vec![]))
            }
        }
    }

    fn abs(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => int64_vec(v.data().iter().map(|n| n.abs()).collect()),
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.abs()).collect()),
            Some(Vector::Int32(v)) => Vector::Int32(types::vector::Int32Vector::from_vec(v.data().iter().map(|n| n.abs()).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn ceil(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.ceil()).collect()),
            Some(Vector::Int64(_)) => args[0].clone(),
            _ => bool_vec(vec![]),
        }
    }

    fn floor(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.floor()).collect()),
            Some(Vector::Int64(_)) => args[0].clone(),
            _ => bool_vec(vec![]),
        }
    }

    fn round(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.round()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn upper(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => string_vec((0..v.len()).map(|i| Some(v.get(i).unwrap_or("").to_uppercase())).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn lower(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => string_vec((0..v.len()).map(|i| Some(v.get(i).unwrap_or("").to_lowercase())).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn length(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| v.get(i).unwrap_or("").len() as i64).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn concat(&self, args: &[Vector]) -> Vector {
        if args.is_empty() { return string_vec(vec![Some(String::new())]); }
        let len = args[0].len();
        let result: Vec<Option<String>> = (0..len).map(|i| {
            let mut s = String::new();
            for arg in args {
                match arg.scalar_at(i) {
                    ScalarValue::String(v) => s.push_str(&v),
                    ScalarValue::Int64(v) => s.push_str(&v.to_string()),
                    ScalarValue::Float64(v) => s.push_str(&v.to_string()),
                    ScalarValue::Null => return None,
                    other => s.push_str(&format!("{:?}", other)),
                }
            }
            Some(s)
        }).collect();
        string_vec(result)
    }

    fn substring(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::String(v)), Some(Vector::Int64(start))) => {
                let result: Vec<Option<String>> = (0..v.len()).map(|i| {
                    let s = v.get(i)?;
                    let st = start.get(i).unwrap_or(1).max(1) as usize;
                    Some(s[st.saturating_sub(1)..].to_string())
                }).collect();
                string_vec(result)
            }
            _ => bool_vec(vec![]),
        }
    }

    fn trim(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => string_vec((0..v.len()).map(|i| Some(v.get(i).unwrap_or("").trim().to_string())).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn coalesce(&self, args: &[Vector]) -> Vector {
        if args.is_empty() { return bool_vec(vec![]); }
        let len = args[0].len();
        let result: Vec<ScalarValue> = (0..len).map(|i| {
            for arg in args {
                let v = arg.scalar_at(i);
                if v != ScalarValue::Null { return v; }
            }
            ScalarValue::Null
        }).collect();
        result.into_iter().next().map(|v| Vector::from_scalar(&v, 1)).unwrap_or_else(|| bool_vec(vec![]))
    }

    fn ifnull(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 { return bool_vec(vec![]); }
        self.coalesce(args)
    }

    fn nullif(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 { return args.first().cloned().unwrap_or_else(|| bool_vec(vec![])); }
        let len = args[0].len();
        let result: Vec<ScalarValue> = (0..len).map(|i| {
            if args[0].scalar_at(i) == args[1].scalar_at(i) { ScalarValue::Null } else { args[0].scalar_at(i) }
        }).collect();
        result.into_iter().next().map(|v| Vector::from_scalar(&v, 1)).unwrap_or_else(|| bool_vec(vec![]))
    }

    fn sum(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => int64_vec(vec![v.data().iter().sum()]),
            Some(Vector::Float64(v)) => float64_vec(vec![v.data().iter().sum()]),
            Some(Vector::Int32(v)) => int64_vec(vec![v.data().iter().map(|&n| n as i64).sum::<i64>()].into_iter().collect()),
            _ => int64_vec(vec![0]),
        }
    }

    fn avg(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => float64_vec(vec![if v.data().is_empty() { 0.0 } else { v.data().iter().sum::<i64>() as f64 / v.data().len() as f64 }]),
            Some(Vector::Float64(v)) => float64_vec(vec![if v.data().is_empty() { 0.0 } else { v.data().iter().sum::<f64>() / v.data().len() as f64 }]),
            _ => float64_vec(vec![0.0]),
        }
    }

    fn min(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => int64_vec(vec![v.data().iter().copied().min().unwrap_or(0)]),
            Some(Vector::Float64(v)) => float64_vec(vec![v.data().iter().copied().fold(f64::INFINITY, f64::min)]),
            Some(Vector::String(v)) => string_vec(vec![(0..v.len()).filter_map(|i| v.get(i)).min().map(|s| s.to_string())]),
            _ => bool_vec(vec![]),
        }
    }

    fn max(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => int64_vec(vec![v.data().iter().copied().max().unwrap_or(0)]),
            Some(Vector::Float64(v)) => float64_vec(vec![v.data().iter().copied().fold(f64::NEG_INFINITY, f64::max)]),
            Some(Vector::String(v)) => string_vec(vec![(0..v.len()).filter_map(|i| v.get(i)).max().map(|s| s.to_string())]),
            _ => bool_vec(vec![]),
        }
    }
}

impl Default for FunctionRegistry { fn default() -> Self { Self::new() } }

fn bool_vec(d: Vec<bool>) -> Vector { Vector::Boolean(types::vector::BooleanVector::from_vec(d)) }
fn int64_vec(d: Vec<i64>) -> Vector { Vector::Int64(types::vector::Int64Vector::from_vec(d)) }
fn float64_vec(d: Vec<f64>) -> Vector { Vector::Float64(types::vector::Float64Vector::from_vec(d)) }
fn string_vec(d: Vec<Option<String>>) -> Vector { Vector::String(types::vector::StringVector::from_option_vec(d)) }

use chrono::{NaiveDate, NaiveDateTime, Datelike, Timelike, Duration, Utc, TimeZone};
use types::{ScalarValue, Vector, JsonValue};

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
            // DATE functions
            "year" => self.year(args),
            "month" => self.month(args),
            "day" | "dayofmonth" => self.day(args),
            "hour" => self.hour(args),
            "minute" => self.minute(args),
            "second" => self.second(args),
            "datediff" => self.datediff(args),
            "curdate" | "current_date" => self.curdate(args),
            "now" | "current_timestamp" => self.now(args),
            "date_add" => self.date_add(args),
            "date_sub" => self.date_sub(args),
            "date_format" => self.date_format(args),
            "date_trunc" => self.date_trunc(args),
            "week" => self.week(args),
            "quarter" => self.quarter(args),
            "monthname" => self.monthname(args),
            "dayname" => self.dayname(args),
            // Math functions
            "sin" => self.sin(args),
            "cos" => self.cos(args),
            "tan" => self.tan(args),
            "asin" | "arcsin" => self.asin(args),
            "acos" | "arccos" => self.acos(args),
            "atan" | "arctan" => self.atan(args),
            "log" | "ln" => self.log(args),
            "log10" => self.log10(args),
            "exp" => self.exp(args),
            "sqrt" => self.sqrt(args),
            "pow" | "power" => self.pow(args),
            "pi" => self.pi(args),
            "e" => self.e(args),
            "rand" | "random" => self.rand(args),
            // Window functions
            "row_number" => self.row_number(args),
            "rank" => self.rank(args),
            "dense_rank" => self.dense_rank(args),
            "lag" => self.lag(args),
            "lead" => self.lead(args),
            // JSON functions
            "json_parse" | "parse_json" => self.json_parse(args),
            "json_query" | "json_extract" => self.json_query(args),
            "json_get" => self.json_get(args),
            "json_contains" => self.json_contains(args),
            "json_array" => self.json_array(args),
            "json_object" => self.json_object(args),
            "json_length" => self.json_length(args),
            "json_keys" => self.json_keys(args),
            "json_valid" => self.json_valid(args),
            // Bitwise functions
            "bitand" | "&" => self.bitand(args),
            "bitor" | "|" => self.bitor(args),
            "bitxor" | "^" => self.bitxor(args),
            "bitnot" | "~" => self.bitnot(args),
            "bitshiftleft" | "<<" => self.bitshiftleft(args),
            "bitshiftright" | ">>" => self.bitshiftright(args),
            // Additional math functions
            "sign" => self.sign(args),
            "degrees" => self.degrees(args),
            "radians" => self.radians(args),
            "truncate" | "trunc" => self.truncate(args),
            "greatest" => self.greatest(args),
            "least" => self.least(args),
            "modulo" | "mod" => self.modulo(args),
            "cot" => self.cot(args),
            "sinh" => self.sinh(args),
            "cosh" => self.cosh(args),
            "tanh" => self.tanh(args),
            // Additional string functions
            "ltrim" => self.ltrim(args),
            "rtrim" => self.rtrim(args),
            "replace" => self.replace(args),
            "left" => self.left(args),
            "right" => self.right(args),
            "locate" | "position" => self.locate(args),
            "repeat" => self.repeat(args),
            "space" => self.space(args),
            "reverse" => self.reverse(args),
            "ascii" => self.ascii(args),
            "char" | "chr" => self.char_func(args),
            "octet_length" => self.octet_length(args),
            "bit_length" => self.bit_length_func(args),
            "concat_ws" => self.concat_ws(args),
            "find_in_set" => self.find_in_set(args),
            "instr" => self.instr(args),
            "lpad" => self.lpad(args),
            "rpad" => self.rpad(args),
            "format" => self.format(args),
            "md5" => self.md5(args),
            "sha1" => self.sha1(args),
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

    // DATE function implementations

    /// YEAR(date) - Extract year from date or datetime
    fn year(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| {
                    naive_date_from_ordinal(d).year() as i64
                }).unwrap_or(0)
            }).collect()),
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| {
                    naive_datetime_from_millis(ms).year() as i64
                }).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| dt.year() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// MONTH(date) - Extract month from date or datetime (1-12)
    fn month(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| naive_date_from_ordinal(d).month() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| naive_datetime_from_millis(ms).month() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| dt.month() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// DAY(date) / DAYOFMONTH(date) - Extract day of month (1-31)
    fn day(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| naive_date_from_ordinal(d).day() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| naive_datetime_from_millis(ms).day() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| dt.day() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// HOUR(datetime) - Extract hour (0-23)
    fn hour(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| naive_datetime_from_millis(ms).hour() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_datetime_string(s)).map(|dt| dt.hour() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// MINUTE(datetime) - Extract minute (0-59)
    fn minute(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| naive_datetime_from_millis(ms).minute() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_datetime_string(s)).map(|dt| dt.minute() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// SECOND(datetime) - Extract second (0-59)
    fn second(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| naive_datetime_from_millis(ms).second() as i64).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_datetime_string(s)).map(|dt| dt.second() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// DATEDIFF(date1, date2) - Returns number of days between two dates
    fn datediff(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return int64_vec(vec![]);
        }
        let len = args[0].len().max(args[1].len());
        let result: Vec<Option<i64>> = (0..len).map(|i| {
            let d1 = args[0].scalar_at(i);
            let d2 = args[1].scalar_at(i);
            let date1 = date_from_scalar(&d1);
            let date2 = date_from_scalar(&d2);
            match (date1, date2) {
                (Some(dt1), Some(dt2)) => Some((dt1 - dt2).num_days()),
                _ => None,
            }
        }).collect();
        int64_vec(result.into_iter().map(|v| v.unwrap_or(0)).collect())
    }

    /// CURDATE() / CURRENT_DATE - Returns current date as ordinal day
    fn curdate(&self, _args: &[Vector]) -> Vector {
        let today = chrono::Local::now().date_naive();
        int64_vec(vec![today.ordinal() as i64])
    }

    /// NOW() / CURRENT_TIMESTAMP - Returns current datetime as milliseconds
    fn now(&self, _args: &[Vector]) -> Vector {
        let now = chrono::Local::now();
        let utc_ts: Option<chrono::DateTime<Utc>> = Utc.from_local_datetime(&now.naive_local()).single();
        int64_vec(vec![utc_ts.map(|dt| dt.timestamp_millis()).unwrap_or(0)])
    }

    /// DATE_ADD(date, INTERVAL expr unit) - Adds interval to date
    fn date_add(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return int64_vec(vec![]);
        }
        let len = args[0].len().max(args[1].len());
        let result: Vec<Option<i64>> = (0..len).map(|i| {
            let date_val = args[0].scalar_at(i);
            let interval_val = args[1].scalar_at(i);
            let date = date_from_scalar(&date_val);
            let interval = parse_interval(&interval_val);
            match (date, interval) {
                (Some(dt), Some(diff)) => {
                    let new_dt = dt + diff;
                    Some(new_dt.timestamp_millis())
                }
                _ => None,
            }
        }).collect();
        int64_vec(result.into_iter().map(|v| v.unwrap_or(0)).collect())
    }

    /// DATE_SUB(date, INTERVAL expr unit) - Subtracts interval from date
    fn date_sub(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return int64_vec(vec![]);
        }
        let len = args[0].len().max(args[1].len());
        let result: Vec<Option<i64>> = (0..len).map(|i| {
            let date_val = args[0].scalar_at(i);
            let interval_val = args[1].scalar_at(i);
            let date = date_from_scalar(&date_val);
            let interval = parse_interval(&interval_val);
            match (date, interval) {
                (Some(dt), Some(diff)) => {
                    let new_dt = dt - diff;
                    Some(new_dt.timestamp_millis())
                }
                _ => None,
            }
        }).collect();
        int64_vec(result.into_iter().map(|v| v.unwrap_or(0)).collect())
    }

    /// DATE_FORMAT(date, format) - Formats date as string
    fn date_format(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return string_vec(vec![None]);
        }
        let format_arg = args.get(1);
        let len = args[0].len().max(format_arg.map(|v| v.len()).unwrap_or(0));
        let result: Vec<Option<String>> = (0..len).map(|i| {
            let date_val = args[0].scalar_at(i);
            let fmt_opt = format_arg.and_then(|v| {
                match v.scalar_at(i) {
                    ScalarValue::String(s) => Some(s),
                    _ => None,
                }
            });
            let dt = datetime_from_scalar(&date_val);
            let fmt = fmt_opt.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S");
            dt.map(|d| format_datetime(&d, fmt))
        }).collect();
        string_vec(result)
    }

    /// DATE_TRUNC(date, unit) - Truncates date to specified unit
    fn date_trunc(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return int64_vec(vec![]);
        }
        let len = args[0].len();
        let result: Vec<Option<i64>> = (0..len).map(|i| {
            let date_val = args[0].scalar_at(i);
            let unit_val = &args[1].scalar_at(i);
            let dt = datetime_from_scalar(&date_val);
            let unit = extract_string(unit_val).unwrap_or_default().to_lowercase();
            dt.map(|d| truncate_datetime(&d, &unit))
        }).collect();
        int64_vec(result.into_iter().map(|v| v.unwrap_or(0)).collect())
    }

    /// WEEK(date) - Returns ISO week number (1-53)
    fn week(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| {
                    let dt = naive_date_from_ordinal(d);
                    dt.iso_week().week() as i64
                }).unwrap_or(0)
            }).collect()),
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| {
                    let dt = naive_datetime_from_millis(ms);
                    dt.iso_week().week() as i64
                }).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| dt.iso_week().week() as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// QUARTER(date) - Returns quarter (1-4)
    fn quarter(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| {
                    let dt = naive_date_from_ordinal(d);
                    ((dt.month() - 1) / 3 + 1) as i64
                }).unwrap_or(0)
            }).collect()),
            Some(Vector::DateTime(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| {
                    let dt = naive_datetime_from_millis(ms);
                    ((dt.month() - 1) / 3 + 1) as i64
                }).unwrap_or(0)
            }).collect()),
            Some(Vector::String(v)) => int64_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| ((dt.month() - 1) / 3 + 1) as i64).unwrap_or(0)
            }).collect()),
            _ => int64_vec(vec![]),
        }
    }

    /// MONTHNAME(date) - Returns month name
    fn monthname(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => string_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| {
                    let dt = naive_date_from_ordinal(d);
                    let month_names = ["January", "February", "March", "April", "May", "June",
                                      "July", "August", "September", "October", "November", "December"];
                    month_names[(dt.month() - 1) as usize].to_string()
                })
            }).collect()),
            Some(Vector::DateTime(v)) => string_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| {
                    let dt = naive_datetime_from_millis(ms);
                    let month_names = ["January", "February", "March", "April", "May", "June",
                                      "July", "August", "September", "October", "November", "December"];
                    month_names[(dt.month() - 1) as usize].to_string()
                })
            }).collect()),
            Some(Vector::String(v)) => string_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| {
                    let month_names = ["January", "February", "March", "April", "May", "June",
                                      "July", "August", "September", "October", "November", "December"];
                    month_names[(dt.month() - 1) as usize].to_string()
                })
            }).collect()),
            _ => string_vec(vec![]),
        }
    }

    /// DAYNAME(date) - Returns day name
    fn dayname(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Date(v)) => string_vec((0..v.len()).map(|i| {
                v.get(i).map(|d| {
                    let dt = naive_date_from_ordinal(d);
                    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
                    day_names[dt.weekday().num_days_from_monday() as usize].to_string()
                })
            }).collect()),
            Some(Vector::DateTime(v)) => string_vec((0..v.len()).map(|i| {
                v.get(i).map(|ms| {
                    let dt = naive_datetime_from_millis(ms);
                    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
                    day_names[dt.weekday().num_days_from_monday() as usize].to_string()
                })
            }).collect()),
            Some(Vector::String(v)) => string_vec((0..v.len()).map(|i| {
                v.get(i).and_then(|s| parse_date_string(s)).map(|dt| {
                    let day_names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
                    day_names[dt.weekday().num_days_from_monday() as usize].to_string()
                })
            }).collect()),
            _ => string_vec(vec![]),
        }
    }
}

// Helper functions

/// Convert ordinal day (day of year) to NaiveDate
/// Using assumption: epoch at day 1 = Jan 1, 1970
fn naive_date_from_ordinal(ordinal: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
        .with_ordinal(ordinal.clamp(1, 366) as u32)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
}

/// Convert milliseconds since epoch to NaiveDateTime
fn naive_datetime_from_millis(ms: i64) -> NaiveDateTime {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.naive_local())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap())
}

/// Parse date string to NaiveDate
fn parse_date_string(s: &str) -> Option<NaiveDate> {
    let formats = ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y", "%Y%m%d"];
    for fmt in &formats {
        if let Ok(dt) = NaiveDate::parse_from_str(s, fmt) {
            return Some(dt);
        }
    }
    None
}

/// Parse datetime string to NaiveDateTime
fn parse_datetime_string(s: &str) -> Option<NaiveDateTime> {
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y/%m/%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y%m%d %H:%M:%S",
    ];
    for fmt in &formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt);
        }
    }
    None
}

/// Extract string from ScalarValue
fn extract_string(val: &ScalarValue) -> Option<String> {
    match val {
        ScalarValue::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Get chrono DateTime from ScalarValue
fn datetime_from_scalar(val: &ScalarValue) -> Option<chrono::DateTime<Utc>> {
    match val {
        ScalarValue::Date(ordinal) => {
            let date = naive_date_from_ordinal(*ordinal);
            Utc.from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap()).single()
        }
        ScalarValue::DateTime(ms) => {
            chrono::DateTime::from_timestamp_millis(*ms)
        }
        ScalarValue::String(s) => {
            parse_datetime_string(s).and_then(|dt| Utc.from_local_datetime(&dt).single())
        }
        _ => None,
    }
}

/// Get chrono DateTime from ScalarValue for date arithmetic
fn date_from_scalar(val: &ScalarValue) -> Option<chrono::DateTime<Utc>> {
    datetime_from_scalar(val)
}

/// Parse interval from scalar value (INTERVAL expr unit)
fn parse_interval(val: &ScalarValue) -> Option<Duration> {
    let s = match val {
        ScalarValue::String(s) => s,
        _ => return None,
    };
    let s = s.trim();
    let s = s.strip_prefix("INTERVAL ").unwrap_or(s);

    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let value: i64 = parts[0].parse().ok()?;
    let unit = parts[1].to_lowercase();

    match unit.as_str() {
        "day" | "days" => Some(Duration::days(value)),
        "month" | "months" => Some(Duration::days(value * 30)),
        "year" | "years" => Some(Duration::days(value * 365)),
        "hour" | "hours" => Some(Duration::hours(value)),
        "minute" | "minutes" => Some(Duration::minutes(value)),
        "second" | "seconds" => Some(Duration::seconds(value)),
        "week" | "weeks" => Some(Duration::weeks(value)),
        _ => None,
    }
}

/// Format datetime according to format string
fn format_datetime(dt: &chrono::DateTime<Utc>, fmt: &str) -> String {
    let naive = dt.naive_local();
    // Simple format strings
    let result = fmt
        .replace("%Y", &naive.format("%Y").to_string())
        .replace("%m", &format!("{:02}", naive.month()))
        .replace("%d", &format!("{:02}", naive.day()))
        .replace("%H", &format!("{:02}", naive.hour()))
        .replace("%M", &format!("{:02}", naive.minute()))
        .replace("%S", &format!("{:02}", naive.second()))
        .replace("%y", &naive.format("%y").to_string())
        .replace("%a", &naive.format("%a").to_string())
        .replace("%b", &naive.format("%b").to_string())
        .replace("%j", &format!("{:03}", naive.ordinal()))
        .replace("%W", &naive.format("%W").to_string())
        .replace("%U", &naive.format("%U").to_string())
        .replace("%p", &naive.format("%p").to_string());
    result
}

/// Truncate datetime to specified unit
fn truncate_datetime(dt: &chrono::DateTime<Utc>, unit: &str) -> i64 {
    let naive = dt.naive_local();
    let truncated = match unit {
        "year" => naive.with_month(1).unwrap().with_day(1).unwrap().with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap(),
        "month" => naive.with_day(1).unwrap().with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap(),
        "day" => naive.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap(),
        "hour" => naive.with_minute(0).unwrap().with_second(0).unwrap(),
        "minute" => naive.with_second(0).unwrap(),
        "second" => naive,
        "week" => {
            let weekday = naive.weekday().num_days_from_monday();
            naive - chrono::Duration::days(weekday as i64)
        },
        _ => naive,
    };
    Utc.from_local_datetime(&truncated).single().unwrap().timestamp_millis()
}

impl Default for FunctionRegistry { fn default() -> Self { Self::new() } }

fn bool_vec(d: Vec<bool>) -> Vector { Vector::Boolean(types::vector::BooleanVector::from_vec(d)) }
fn int64_vec(d: Vec<i64>) -> Vector { Vector::Int64(types::vector::Int64Vector::from_vec(d)) }
fn float64_vec(d: Vec<f64>) -> Vector { Vector::Float64(types::vector::Float64Vector::from_vec(d)) }
fn string_vec(d: Vec<Option<String>>) -> Vector { Vector::String(types::vector::StringVector::from_option_vec(d)) }

// Mathematical functions (trigonometric, logarithmic, exponential)
impl FunctionRegistry {
    // Trigonometric functions
    fn sin(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.to_radians().sin()).collect()),
            Some(Vector::Int64(v)) => float64_vec(v.data().iter().map(|n| (*n as f64).to_radians().sin()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn cos(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.to_radians().cos()).collect()),
            Some(Vector::Int64(v)) => float64_vec(v.data().iter().map(|n| (*n as f64).to_radians().cos()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn tan(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.to_radians().tan()).collect()),
            Some(Vector::Int64(v)) => float64_vec(v.data().iter().map(|n| (*n as f64).to_radians().tan()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn asin(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.asin().to_degrees()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn acos(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.acos().to_degrees()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn atan(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.atan().to_degrees()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    // Logarithmic functions
    fn log(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.ln()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    fn log10(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.log10()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    // Exponential function
    fn exp(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.exp()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    // Square root
    fn sqrt(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.sqrt()).collect()),
            _ => bool_vec(vec![]),
        }
    }

    // Power function
    fn pow(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return bool_vec(vec![]);
        }
        match (&args[0], &args[1]) {
            (Vector::Float64(base), Vector::Float64(exp)) => {
                let result: Vec<f64> = base.data().iter().zip(exp.data().iter())
                    .map(|(b, e)| b.powf(*e))
                    .collect();
                float64_vec(result)
            }
            _ => bool_vec(vec![]),
        }
    }

    // Constants
    fn pi(&self, _args: &[Vector]) -> Vector {
        float64_vec(vec![std::f64::consts::PI])
    }

    fn e(&self, _args: &[Vector]) -> Vector {
        float64_vec(vec![std::f64::consts::E])
    }

    // Random number (simple pseudo-random implementation)
    fn rand(&self, args: &[Vector]) -> Vector {
        let count = args.first().map(|v| v.len()).unwrap_or(1);
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        float64_vec((0..count).enumerate().map(|(i, _)| {
            // Simple linear congruential generator
            let state = seed.wrapping_add(i as u64);
            ((state as f64) / (u64::MAX as f64))
        }).collect())
    }

    // Window functions (basic implementations)
    fn row_number(&self, args: &[Vector]) -> Vector {
        let count = args.first().map(|v| v.len()).unwrap_or(1);
        int64_vec((1..=count as i64).collect())
    }

    fn rank(&self, args: &[Vector]) -> Vector {
        // Simplified rank implementation - same as row_number for now
        let count = args.first().map(|v| v.len()).unwrap_or(1);
        int64_vec((1..=count as i64).collect())
    }

    fn dense_rank(&self, args: &[Vector]) -> Vector {
        // Simplified dense_rank - same as row_number for now
        let count = args.first().map(|v| v.len()).unwrap_or(1);
        int64_vec((1..=count as i64).collect())
    }

    fn lag(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => {
                let offset = args.get(1).and_then(|a| {
                    if let Vector::Int64(o) = a { o.data().first() } else { None }
                }).unwrap_or(&1);
                let default = args.get(2).and_then(|a| {
                    if let Vector::Int64(d) = a { d.data().first() } else { None }
                }).unwrap_or(&0);

                int64_vec(v.data().iter().enumerate().map(|(i, &val)| {
                    if i >= *offset as usize {
                        v.data()[i - *offset as usize]
                    } else {
                        *default
                    }
                }).collect())
            }
            Some(Vector::Float64(v)) => {
                let offset = args.get(1).and_then(|a| {
                    if let Vector::Int64(o) = a { o.data().first() } else { None }
                }).unwrap_or(&1);
                let default = args.get(2).and_then(|a| {
                    if let Vector::Float64(d) = a { d.data().first() } else { None }
                }).unwrap_or(&0.0);

                float64_vec(v.data().iter().enumerate().map(|(i, &val)| {
                    if i >= *offset as usize {
                        v.data()[i - *offset as usize]
                    } else {
                        *default
                    }
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn lead(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => {
                let offset = args.get(1).and_then(|a| {
                    if let Vector::Int64(o) = a { o.data().first() } else { None }
                }).unwrap_or(&1);
                let default = args.get(2).and_then(|a| {
                    if let Vector::Int64(d) = a { d.data().first() } else { None }
                }).unwrap_or(&0);

                int64_vec(v.data().iter().enumerate().map(|(i, &val)| {
                    let next_idx = i + *offset as usize;
                    if next_idx < v.data().len() {
                        v.data()[next_idx]
                    } else {
                        *default
                    }
                }).collect())
            }
            Some(Vector::Float64(v)) => {
                let offset = args.get(1).and_then(|a| {
                    if let Vector::Int64(o) = a { o.data().first() } else { None }
                }).unwrap_or(&1);
                let default = args.get(2).and_then(|a| {
                    if let Vector::Float64(d) = a { d.data().first() } else { None }
                }).unwrap_or(&0.0);

                float64_vec(v.data().iter().enumerate().map(|(i, &val)| {
                    let next_idx = i + *offset as usize;
                    if next_idx < v.data().len() {
                        v.data()[next_idx]
                    } else {
                        *default
                    }
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    // JSON functions
    fn json_parse(&self, args: &[Vector]) -> Vector {
        use types::{Vector as TypesVector, JsonVector, JsonValue};

        match args.first() {
            Some(Vector::String(v)) => {
                let json_values: Vec<types::ScalarValue> = (0..v.len()).map(|i| {
                    if let Some(s) = v.get(i) {
                        self.parse_json_string(s)
                    } else {
                        types::ScalarValue::Json(JsonValue::Null)
                    }
                }).collect();
                TypesVector::Json(JsonVector::from_vec(json_values))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn parse_json_string(&self, s: &str) -> types::ScalarValue {
        use types::JsonValue;

        // Simple JSON parser - handles basic cases
        let trimmed = s.trim();
        if trimmed == "null" {
            return types::ScalarValue::Json(JsonValue::Null);
        }
        if trimmed == "true" {
            return types::ScalarValue::Json(JsonValue::Bool(true));
        }
        if trimmed == "false" {
            return types::ScalarValue::Json(JsonValue::Bool(false));
        }
        if let Some(num_str) = trimmed.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
            return types::ScalarValue::Json(JsonValue::String(num_str.to_string()));
        }
        if let Ok(n) = trimmed.parse::<f64>() {
            return types::ScalarValue::Json(JsonValue::Number(n));
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Array
            let inner = &trimmed[1..trimmed.len()-1];
            let items: Vec<JsonValue> = if inner.is_empty() {
                vec![]
            } else {
                inner.split(',').filter_map(|item| {
                    if let types::ScalarValue::Json(j) = self.parse_json_string(item.trim()) {
                        Some(j)
                    } else {
                        None
                    }
                }).collect()
            };
            return types::ScalarValue::Json(JsonValue::Array(items));
        }
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            // Object
            let inner = &trimmed[1..trimmed.len()-1];
            let pairs: Vec<(String, JsonValue)> = if inner.is_empty() {
                vec![]
            } else {
                inner.split(',').filter_map(|pair| {
                    let parts: Vec<&str> = pair.split(':').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim().trim_matches('"');
                        if let types::ScalarValue::Json(val) = self.parse_json_string(parts[1].trim()) {
                            Some((key.to_string(), val))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }).collect()
            };
            return types::ScalarValue::Json(JsonValue::Object(pairs));
        }

        // Fallback for invalid JSON
        types::ScalarValue::Null
    }

    fn json_query(&self, args: &[Vector]) -> Vector {
        use types::{Vector as TypesVector, JsonVector};

        match (args.first(), args.get(1)) {
            (Some(Vector::Json(json_vec)), Some(Vector::String(path_vec))) => {
                let results: Vec<types::ScalarValue> = (0..json_vec.len()).map(|i| {
                    if let (Some(json), Some(path)) = (json_vec.get(i), path_vec.get(i)) {
                        self.json_extract(&json, path)
                    } else {
                        types::ScalarValue::Null
                    }
                }).collect();
                TypesVector::Json(JsonVector::from_vec(results))
            }
            (Some(Vector::String(json_vec)), Some(Vector::String(path_vec))) => {
                let results: Vec<types::ScalarValue> = (0..json_vec.len()).map(|i| {
                    if let (Some(json_str), Some(path)) = (json_vec.get(i), path_vec.get(i)) {
                        let json_val = self.parse_json_string(json_str);
                        self.json_extract(&json_val, path)
                    } else {
                        types::ScalarValue::Null
                    }
                }).collect();
                TypesVector::Json(JsonVector::from_vec(results))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn json_extract(&self, json: &types::ScalarValue, path: &str) -> types::ScalarValue {
        use types::JsonValue;

        let json_val = match json {
            types::ScalarValue::Json(j) => j,
            _ => return types::ScalarValue::Null,
        };

        // Simple JSONPath-like extraction: $.key, $.array[index], $."key with spaces"
        let path_parts: Vec<&str> = path.split('.').skip(1).collect();
        let mut current = json_val;

        for part in path_parts {
            current = match current {
                JsonValue::Object(pairs) => {
                    let key = part.trim_matches('"');
                    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v).unwrap_or(&JsonValue::Null)
                }
                JsonValue::Array(items) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        items.get(idx).unwrap_or(&JsonValue::Null)
                    } else {
                        &JsonValue::Null
                    }
                }
                _ => &JsonValue::Null,
            };
        }

        types::ScalarValue::Json(current.clone())
    }

    fn json_get(&self, args: &[Vector]) -> Vector {
        // Alias for json_query with simpler key access
        self.json_query(args)
    }

    fn json_contains(&self, args: &[Vector]) -> Vector {
        use types::JsonValue;

        match (args.first(), args.get(1)) {
            (Some(Vector::Json(json_vec)), Some(target)) => {
                let target_len = target.len();
                let results: Vec<bool> = (0..json_vec.len()).map(|i| {
                    let target_val = target.scalar_at(i);
                    let target_str = match target_val {
                        types::ScalarValue::Json(JsonValue::String(s)) => s,
                        types::ScalarValue::String(s) => s,
                        types::ScalarValue::Json(JsonValue::Number(n)) => n.to_string(),
                        types::ScalarValue::Json(JsonValue::Bool(b)) => b.to_string(),
                        types::ScalarValue::Int64(n) => n.to_string(),
                        types::ScalarValue::Float64(n) => n.to_string(),
                        _ => return false,
                    };
                    match json_vec.get(i) {
                        Some(types::ScalarValue::Json(json)) => {
                            self.json_value_contains(&json, &target_str)
                        }
                        _ => false,
                    }
                }).collect();
                bool_vec(results)
            }
            (Some(Vector::String(json_vec)), Some(target)) => {
                let results: Vec<bool> = (0..json_vec.len()).map(|i| {
                    let target_val = target.scalar_at(i);
                    let target_str = match target_val {
                        types::ScalarValue::Json(JsonValue::String(s)) => s,
                        types::ScalarValue::String(s) => s,
                        types::ScalarValue::Json(JsonValue::Number(n)) => n.to_string(),
                        types::ScalarValue::Json(JsonValue::Bool(b)) => b.to_string(),
                        types::ScalarValue::Int64(n) => n.to_string(),
                        types::ScalarValue::Float64(n) => n.to_string(),
                        _ => return false,
                    };
                    if let Some(s) = json_vec.get(i) {
                        if let types::ScalarValue::Json(json) = self.parse_json_string(s) {
                            self.json_value_contains(&json, &target_str)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }).collect();
                bool_vec(results)
            }
            _ => bool_vec(vec![]),
        }
    }

    fn json_value_contains(&self, json: &JsonValue, target: &str) -> bool {
        match json {
            JsonValue::String(s) => s.contains(target),
            JsonValue::Array(items) => items.iter().any(|item| self.json_value_contains(item, target)),
            JsonValue::Object(pairs) => pairs.iter().any(|(_, v)| self.json_value_contains(v, target)),
            JsonValue::Number(n) => target == n.to_string(),
            JsonValue::Bool(b) => target == b.to_string(),
            JsonValue::Null => false,
        }
    }

    fn json_array(&self, args: &[Vector]) -> Vector {
        use types::{Vector as TypesVector, JsonVector, JsonValue};

        let items: Vec<JsonValue> = args.iter().flat_map(|arg| (0..arg.len()).map(|i| {
            match arg.scalar_at(i) {
                types::ScalarValue::Null => JsonValue::Null,
                types::ScalarValue::String(s) => JsonValue::String(s),
                types::ScalarValue::Int64(n) => JsonValue::Number(n as f64),
                types::ScalarValue::Float64(n) => JsonValue::Number(n),
                types::ScalarValue::Boolean(b) => JsonValue::Bool(b),
                types::ScalarValue::Json(j) => j.clone(),
                _ => JsonValue::Null,
            }
        })).collect();

        TypesVector::Json(JsonVector::from_vec(vec![types::ScalarValue::Json(JsonValue::Array(items))]))
    }

    fn json_object(&self, args: &[Vector]) -> Vector {
        use types::{Vector as TypesVector, JsonVector, JsonValue};

        if args.len() % 2 != 0 {
            return bool_vec(vec![]);
        }

        let len = args.first().map(|v| v.len()).unwrap_or(1);
        let result = types::ScalarValue::Json(JsonValue::Array((0..len).map(|i| {
            let mut pairs = vec![];

            for chunk in args.chunks(2) {
                if let (Some(key_vec), Some(val_vec)) = (chunk.first(), chunk.get(1)) {
                    if let (types::ScalarValue::String(key), val) = (key_vec.scalar_at(i), val_vec.scalar_at(i)) {
                        let json_val = match val {
                            types::ScalarValue::Null => JsonValue::Null,
                            types::ScalarValue::String(s) => JsonValue::String(s.clone()),
                            types::ScalarValue::Int64(n) => JsonValue::Number(n as f64),
                            types::ScalarValue::Float64(n) => JsonValue::Number(n),
                            types::ScalarValue::Boolean(b) => JsonValue::Bool(b),
                            types::ScalarValue::Json(j) => j.clone(),
                            _ => JsonValue::Null,
                        };
                        pairs.push((key.clone(), json_val));
                    }
                }
            }

            JsonValue::Object(pairs)
        }).collect()));

        TypesVector::Json(JsonVector::from_vec(vec![result]))
    }

    fn json_length(&self, args: &[Vector]) -> Vector {
        use types::JsonValue;

        match args.first() {
            Some(Vector::Json(v)) => {
                let lengths: Vec<i64> = (0..v.len()).map(|i| {
                    match v.get(i) {
                        Some(types::ScalarValue::Json(json)) => self.json_value_length(&json) as i64,
                        _ => 0,
                    }
                }).collect();
                int64_vec(lengths)
            }
            Some(Vector::String(v)) => {
                let lengths: Vec<i64> = (0..v.len()).map(|i| {
                    if let Some(s) = v.get(i) {
                        if let types::ScalarValue::Json(json) = self.parse_json_string(s) {
                            self.json_value_length(&json) as i64
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                }).collect();
                int64_vec(lengths)
            }
            _ => bool_vec(vec![]),
        }
    }

    fn json_value_length(&self, json: &JsonValue) -> usize {
        match json {
            JsonValue::Null => 1,
            JsonValue::Bool(_) => 1,
            JsonValue::Number(_) => 1,
            JsonValue::String(s) => s.len(),
            JsonValue::Array(items) => items.len(),
            JsonValue::Object(pairs) => pairs.len(),
        }
    }

    fn json_keys(&self, args: &[Vector]) -> Vector {
        use types::{Vector as TypesVector, JsonVector, JsonValue};

        // json_keys can take JSON string or direct JSON
        match args.first() {
            Some(Vector::Json(v)) => {
                let key_arrays: Vec<types::ScalarValue> = (0..v.len()).map(|i| {
                    match v.get(i) {
                        Some(types::ScalarValue::Json(JsonValue::Object(pairs))) => {
                            let keys: Vec<JsonValue> = pairs.iter().map(|(k, _)| JsonValue::String(k.clone())).collect();
                            types::ScalarValue::Json(JsonValue::Array(keys))
                        }
                        _ => types::ScalarValue::Json(JsonValue::Array(vec![])),
                    }
                }).collect();
                TypesVector::Json(JsonVector::from_vec(key_arrays))
            }
            Some(Vector::String(v)) => {
                let key_arrays: Vec<types::ScalarValue> = (0..v.len()).map(|i| {
                    if let Some(s) = v.get(i) {
                        if let types::ScalarValue::Json(JsonValue::Object(pairs)) = self.parse_json_string(s) {
                            let keys: Vec<JsonValue> = pairs.iter().map(|(k, _)| JsonValue::String(k.clone())).collect();
                            types::ScalarValue::Json(JsonValue::Array(keys))
                        } else {
                            types::ScalarValue::Json(JsonValue::Array(vec![]))
                        }
                    } else {
                        types::ScalarValue::Json(JsonValue::Array(vec![]))
                    }
                }).collect();
                TypesVector::Json(JsonVector::from_vec(key_arrays))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn json_valid(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                let results: Vec<bool> = (0..v.len()).map(|i| {
                    v.get(i).map(|s| self.is_valid_json(s)).unwrap_or(false)
                }).collect();
                bool_vec(results)
            }
            Some(Vector::Json(v)) => {
                // Already parsed JSON is valid
                bool_vec((0..v.len()).map(|_| true).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn is_valid_json(&self, s: &str) -> bool {
        let trimmed = s.trim();

        if trimmed == "null" || trimmed == "true" || trimmed == "false" {
            return true;
        }

        // Check for string
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            return true;
        }

        // Check for number
        if trimmed.parse::<f64>().is_ok() {
            return true;
        }

        // Check for array
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            return true; // Simple check, could validate contents
        }

        // Check for object
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            return true; // Simple check, could validate contents
        }

        false
    }

    // Bitwise functions
    fn bitand(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Int64(a)), Some(Vector::Int64(b))) => {
                let len = a.len().min(b.len());
                int64_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) & b.get(i).unwrap_or(0)
                }).collect())
            }
            (Some(Vector::Int32(a)), Some(Vector::Int32(b))) => {
                let len = a.len().min(b.len());
                Vector::Int32(types::vector::Int32Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) & b.get(i).unwrap_or(0)
                }).collect()))
            }
            (Some(Vector::Int16(a)), Some(Vector::Int16(b))) => {
                let len = a.len().min(b.len());
                Vector::Int16(types::vector::Int16Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) & b.get(i).unwrap_or(0)
                }).collect()))
            }
            (Some(Vector::Int8(a)), Some(Vector::Int8(b))) => {
                let len = a.len().min(b.len());
                Vector::Int8(types::vector::Int8Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) & b.get(i).unwrap_or(0)
                }).collect()))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn bitor(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Int64(a)), Some(Vector::Int64(b))) => {
                let len = a.len().min(b.len());
                int64_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) | b.get(i).unwrap_or(0)
                }).collect())
            }
            (Some(Vector::Int32(a)), Some(Vector::Int32(b))) => {
                let len = a.len().min(b.len());
                Vector::Int32(types::vector::Int32Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) | b.get(i).unwrap_or(0)
                }).collect()))
            }
            (Some(Vector::Int16(a)), Some(Vector::Int16(b))) => {
                let len = a.len().min(b.len());
                Vector::Int16(types::vector::Int16Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) | b.get(i).unwrap_or(0)
                }).collect()))
            }
            (Some(Vector::Int8(a)), Some(Vector::Int8(b))) => {
                let len = a.len().min(b.len());
                Vector::Int8(types::vector::Int8Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) | b.get(i).unwrap_or(0)
                }).collect()))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn bitxor(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Int64(a)), Some(Vector::Int64(b))) => {
                let len = a.len().min(b.len());
                int64_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) ^ b.get(i).unwrap_or(0)
                }).collect())
            }
            (Some(Vector::Int32(a)), Some(Vector::Int32(b))) => {
                let len = a.len().min(b.len());
                Vector::Int32(types::vector::Int32Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) ^ b.get(i).unwrap_or(0)
                }).collect()))
            }
            (Some(Vector::Int16(a)), Some(Vector::Int16(b))) => {
                let len = a.len().min(b.len());
                Vector::Int16(types::vector::Int16Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) ^ b.get(i).unwrap_or(0)
                }).collect()))
            }
            (Some(Vector::Int8(a)), Some(Vector::Int8(b))) => {
                let len = a.len().min(b.len());
                Vector::Int8(types::vector::Int8Vector::from_vec((0..len).map(|i| {
                    a.get(i).unwrap_or(0) ^ b.get(i).unwrap_or(0)
                }).collect()))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn bitnot(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => int64_vec(v.data().iter().map(|n| !n).collect()),
            Some(Vector::Int32(v)) => Vector::Int32(types::vector::Int32Vector::from_vec(v.data().iter().map(|n| !n).collect())),
            Some(Vector::Int16(v)) => Vector::Int16(types::vector::Int16Vector::from_vec(v.data().iter().map(|n| !n).collect())),
            Some(Vector::Int8(v)) => Vector::Int8(types::vector::Int8Vector::from_vec(v.data().iter().map(|n| !n).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn bitshiftleft(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Int64(a)), Some(Vector::Int64(b))) => {
                let len = a.len().min(b.len());
                int64_vec((0..len).map(|i| {
                    let shift = b.get(i).unwrap_or(0);
                    a.get(i).unwrap_or(0) << shift
                }).collect())
            }
            (Some(Vector::Int32(a)), Some(Vector::Int32(b))) => {
                let len = a.len().min(b.len());
                Vector::Int32(types::vector::Int32Vector::from_vec((0..len).map(|i| {
                    let shift = b.get(i).unwrap_or(0);
                    a.get(i).unwrap_or(0) << shift
                }).collect()))
            }
            _ => bool_vec(vec![]),
        }
    }

    fn bitshiftright(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Int64(a)), Some(Vector::Int64(b))) => {
                let len = a.len().min(b.len());
                int64_vec((0..len).map(|i| {
                    let shift = b.get(i).unwrap_or(0);
                    a.get(i).unwrap_or(0) >> shift
                }).collect())
            }
            (Some(Vector::Int32(a)), Some(Vector::Int32(b))) => {
                let len = a.len().min(b.len());
                Vector::Int32(types::vector::Int32Vector::from_vec((0..len).map(|i| {
                    let shift = b.get(i).unwrap_or(0);
                    a.get(i).unwrap_or(0) >> shift
                }).collect()))
            }
            _ => bool_vec(vec![]),
        }
    }

    // Additional math functions
    fn sign(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => int64_vec(v.data().iter().map(|n| {
                if n > &0.0 { 1 } else if n < &0.0 { -1 } else { 0 }
            }).collect()),
            Some(Vector::Float32(v)) => Vector::Int32(types::vector::Int32Vector::from_vec(v.data().iter().map(|n| {
                if n > &0.0 { 1 } else if n < &0.0 { -1 } else { 0 }
            }).collect())),
            Some(Vector::Int64(v)) => int64_vec(v.data().iter().map(|n| {
                if n > &0 { 1 } else if n < &0 { -1 } else { 0 }
            }).collect()),
            Some(Vector::Int32(v)) => Vector::Int32(types::vector::Int32Vector::from_vec(v.data().iter().map(|n| {
                if n > &0 { 1 } else if n < &0 { -1 } else { 0 }
            }).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn degrees(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.to_degrees()).collect()),
            Some(Vector::Float32(v)) => Vector::Float32(types::vector::Float32Vector::from_vec(v.data().iter().map(|n| n.to_degrees()).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn radians(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.to_radians()).collect()),
            Some(Vector::Float32(v)) => Vector::Float32(types::vector::Float32Vector::from_vec(v.data().iter().map(|n| n.to_radians()).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn truncate(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Float64(v)), Some(Vector::Int64(d))) => {
                let len = v.len().min(d.len());
                float64_vec((0..len).map(|i| {
                    let n = v.get(i).unwrap_or(0.0);
                    let decimals = d.get(i).unwrap_or(0) as i32;
                    let multiplier = 10_f64.powi(decimals);
                    let sign = if n >= 0.0 { 1.0 } else { -1.0 };
                    sign * (n.abs() * multiplier).floor() / multiplier
                }).collect())
            }
            (Some(Vector::Float64(v)), None) => float64_vec(v.data().iter().map(|n| n.trunc()).collect()),
            (Some(Vector::Int64(v)), _) => int64_vec(v.data().to_vec()),
            (Some(Vector::Int32(v)), _) => Vector::Int32(types::vector::Int32Vector::from_vec(v.data().to_vec())),
            _ => bool_vec(vec![]),
        }
    }

    fn greatest(&self, args: &[Vector]) -> Vector {
        if args.is_empty() {
            return bool_vec(vec![]);
        }

        let len = args[0].len();
        let mut result = vec![];

        for i in 0..len {
            let mut max_val = args[0].scalar_at(i);

            for arg in &args[1..] {
                let current = arg.scalar_at(i);
                match (&max_val, &current) {
                    (ScalarValue::Null, _) => max_val = current,
                    (ScalarValue::Int64(a), ScalarValue::Int64(b)) if b > a => max_val = ScalarValue::Int64(*b),
                    (ScalarValue::Float64(a), ScalarValue::Float64(b)) if b > a => max_val = ScalarValue::Float64(*b),
                    (ScalarValue::String(a), ScalarValue::String(b)) if b.as_str() > a.as_str() => max_val = ScalarValue::String(b.clone()),
                    _ => {}
                }
            }

            result.push(max_val);
        }

        Vector::from_scalar(&result.first().unwrap_or(&ScalarValue::Null), 0)
            .slice(0, 0)
            .filter(&types::Bitmap::with_capacity(len));

        match result.first() {
            Some(ScalarValue::Int64(_)) => {
                let vals: Vec<i64> = result.into_iter().filter_map(|v| match v {
                    ScalarValue::Int64(n) => Some(n),
                    _ => None
                }).collect();
                int64_vec(vals)
            }
            Some(ScalarValue::Float64(_)) => {
                let vals: Vec<f64> = result.into_iter().filter_map(|v| match v {
                    ScalarValue::Float64(n) => Some(n),
                    _ => None
                }).collect();
                float64_vec(vals)
            }
            Some(ScalarValue::String(_)) => {
                let vals: Vec<String> = result.into_iter().filter_map(|v| match v {
                    ScalarValue::String(s) => Some(s),
                    _ => None
                }).collect();
                string_vec(vals.into_iter().map(Some).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn least(&self, args: &[Vector]) -> Vector {
        if args.is_empty() {
            return bool_vec(vec![]);
        }

        let len = args[0].len();
        let mut result = vec![];

        for i in 0..len {
            let mut min_val = args[0].scalar_at(i);

            for arg in &args[1..] {
                let current = arg.scalar_at(i);
                match (&min_val, &current) {
                    (ScalarValue::Null, _) => min_val = current,
                    (ScalarValue::Int64(a), ScalarValue::Int64(b)) if b < a => min_val = ScalarValue::Int64(*b),
                    (ScalarValue::Float64(a), ScalarValue::Float64(b)) if b < a => min_val = ScalarValue::Float64(*b),
                    (ScalarValue::String(a), ScalarValue::String(b)) if b.as_str() < a.as_str() => min_val = ScalarValue::String(b.clone()),
                    _ => {}
                }
            }

            result.push(min_val);
        }

        match result.first() {
            Some(ScalarValue::Int64(_)) => {
                let vals: Vec<i64> = result.into_iter().filter_map(|v| match v {
                    ScalarValue::Int64(n) => Some(n),
                    _ => None
                }).collect();
                int64_vec(vals)
            }
            Some(ScalarValue::Float64(_)) => {
                let vals: Vec<f64> = result.into_iter().filter_map(|v| match v {
                    ScalarValue::Float64(n) => Some(n),
                    _ => None
                }).collect();
                float64_vec(vals)
            }
            Some(ScalarValue::String(_)) => {
                let vals: Vec<String> = result.into_iter().filter_map(|v| match v {
                    ScalarValue::String(s) => Some(s),
                    _ => None
                }).collect();
                string_vec(vals.into_iter().map(Some).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn modulo(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Int64(a)), Some(Vector::Int64(b))) => {
                let len = a.len().min(b.len());
                int64_vec((0..len).map(|i| {
                    let dividend = a.get(i).unwrap_or(0);
                    let divisor = b.get(i).unwrap_or(1);
                    if divisor == 0 { 0 } else { dividend % divisor }
                }).collect())
            }
            (Some(Vector::Float64(a)), Some(Vector::Float64(b))) => {
                let len = a.len().min(b.len());
                float64_vec((0..len).map(|i| {
                    let dividend = a.get(i).unwrap_or(0.0);
                    let divisor = b.get(i).unwrap_or(1.0);
                    if divisor == 0.0 { 0.0 } else { dividend % divisor }
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn cot(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| {
                1.0 / n.tan()
            }).collect()),
            Some(Vector::Float32(v)) => Vector::Float32(types::vector::Float32Vector::from_vec(v.data().iter().map(|n| {
                1.0 / n.tan()
            }).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn sinh(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.sinh()).collect()),
            Some(Vector::Float32(v)) => Vector::Float32(types::vector::Float32Vector::from_vec(v.data().iter().map(|n| n.sinh()).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn cosh(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.cosh()).collect()),
            Some(Vector::Float32(v)) => Vector::Float32(types::vector::Float32Vector::from_vec(v.data().iter().map(|n| n.cosh()).collect())),
            _ => bool_vec(vec![]),
        }
    }

    fn tanh(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Float64(v)) => float64_vec(v.data().iter().map(|n| n.tanh()).collect()),
            Some(Vector::Float32(v)) => Vector::Float32(types::vector::Float32Vector::from_vec(v.data().iter().map(|n| n.tanh()).collect())),
            _ => bool_vec(vec![]),
        }
    }

    // Additional string functions
    fn ltrim(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                let results: Vec<String> = (0..v.len()).map(|i| {
                    v.get(i).unwrap_or("").trim_start().to_string()
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn rtrim(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                let results: Vec<String> = (0..v.len()).map(|i| {
                    v.get(i).unwrap_or("").trim_end().to_string()
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn replace(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1), args.get(2)) {
            (Some(Vector::String(v)), Some(Vector::String(from)), Some(Vector::String(to))) => {
                let len = v.len().min(from.len()).min(to.len());
                let results: Vec<String> = (0..len).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let from_str = from.get(i).unwrap_or("");
                    let to_str = to.get(i).unwrap_or("");
                    s.replace(from_str, to_str)
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn left(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::String(v)), Some(Vector::Int64(len_vec))) => {
                let results: Vec<String> = (0..v.len().min(len_vec.len())).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let len = len_vec.get(i).unwrap_or(0) as usize;
                    s.chars().take(len).collect()
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn right(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::String(v)), Some(Vector::Int64(len_vec))) => {
                let results: Vec<String> = (0..v.len().min(len_vec.len())).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let len = len_vec.get(i).unwrap_or(0) as usize;
                    s.chars().rev().take(len).collect::<String>().chars().rev().collect()
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn locate(&self, args: &[Vector]) -> Vector {
        // locate(substring, string, [start_position])
        // Returns 1-based position of first occurrence, or 0 if not found
        // Note: SQL LOCATE(substring, string) - first arg is needle, second is haystack
        match (args.get(0), args.get(1), args.get(2)) {
            (Some(Vector::String(needle)), Some(Vector::String(haystack)), None) => {
                let len = haystack.len();
                int64_vec((0..len).map(|i| {
                    let hay = haystack.get(i).unwrap_or("");
                    let need = needle.get(i).unwrap_or("");
                    hay.find(need).map(|pos| pos as i64 + 1).unwrap_or(0)
                }).collect())
            }
            (Some(Vector::String(needle)), Some(Vector::String(haystack)), Some(Vector::Int64(start))) => {
                let len = haystack.len();
                int64_vec((0..len).map(|i| {
                    let hay = haystack.get(i).unwrap_or("");
                    let need = needle.get(i).unwrap_or("");
                    let start_pos = start.get(i).unwrap_or(1) as usize;
                    if start_pos > 0 && start_pos <= hay.len() {
                        hay[start_pos - 1..].find(need).map(|pos| pos as i64 + start_pos as i64).unwrap_or(0)
                    } else {
                        0
                    }
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn repeat(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::String(v)), Some(Vector::Int64(count))) => {
                let len = v.len().min(count.len());
                let results: Vec<String> = (0..len).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let n = count.get(i).unwrap_or(1).max(0) as usize;
                    s.repeat(n)
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn space(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::Int64(v)) => {
                let results: Vec<String> = v.data().iter().map(|n| {
                    " ".repeat((*n.max(&0)) as usize)
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn reverse(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                let results: Vec<String> = (0..v.len()).map(|i| {
                    v.get(i).unwrap_or("").chars().rev().collect()
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn ascii(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                int64_vec((0..v.len()).map(|i| {
                    v.get(i).unwrap_or("").chars().next().map(|c| c as i64).unwrap_or(0)
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn char_func(&self, args: &[Vector]) -> Vector {
        if args.is_empty() {
            return string_vec(vec![]);
        }

        let len = args[0].len();
        let results: Vec<String> = (0..len).map(|i| {
args.iter().filter_map(|v| {
                if let Vector::Int64(sv) = v {
                    sv.get(i).map(|n| {
                        let ch = (n.clamp(0, 255) as u8) as char;
                        if ch.is_ascii() { Some(ch) } else { Some('\u{FFFD}') }
                    }).flatten()
                } else {
                    None
                }
            }).collect()
        }).collect();
        string_vec(results.iter().map(|s| Some(s.clone())).collect())
    }

    fn octet_length(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                int64_vec((0..v.len()).map(|i| {
                    v.get(i).unwrap_or("").len() as i64
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn bit_length_func(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                int64_vec((0..v.len()).map(|i| {
                    v.get(i).unwrap_or("").len() as i64 * 8
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn concat_ws(&self, args: &[Vector]) -> Vector {
        if args.len() < 2 {
            return bool_vec(vec![]);
        }

        let sep_vec = &args[0];
        let str_vecs = &args[1..];

        let len = sep_vec.len();

        let results: Vec<String> = (0..len).map(|i| {
            let parts: Vec<&str> = str_vecs.iter().filter_map(|v| {
                if let Vector::String(sv) = v {
                    sv.get(i)
                } else {
                    None
                }
            }).collect();

            if let Vector::String(sep) = sep_vec {
                let sep_str = sep.get(i).unwrap_or("");
                parts.join(sep_str)
            } else {
                parts.join("")
            }
        }).collect();

        string_vec(results.iter().map(|s| Some(s.clone())).collect())
    }

    fn find_in_set(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::String(str)), Some(Vector::String(strlist))) => {
                let len = str.len().min(strlist.len());
                int64_vec((0..len).map(|i| {
                    let s = str.get(i).unwrap_or("");
                    let list = strlist.get(i).unwrap_or("");
                    list.split(',').position(|part| part.trim() == s).map(|pos| pos as i64 + 1).unwrap_or(0)
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn instr(&self, args: &[Vector]) -> Vector {
        // instr(str, substr) - returns 1-based position
        match (args.first(), args.get(1)) {
            (Some(Vector::String(v)), Some(Vector::String(sub))) => {
                let len = v.len().min(sub.len());
                int64_vec((0..len).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let sub_str = sub.get(i).unwrap_or("");
                    s.find(sub_str).map(|pos| pos as i64 + 1).unwrap_or(0)
                }).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn lpad(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1), args.get(2)) {
            (Some(Vector::String(v)), Some(Vector::Int64(len)), Some(Vector::String(pad))) => {
                let results_len = v.len().min(len.len()).min(pad.len());
                let results: Vec<String> = (0..results_len).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let target_len = len.get(i).unwrap_or(0) as usize;
                    let pad_str = pad.get(i).unwrap_or(" ");
                    if s.len() >= target_len {
                        s.chars().take(target_len).collect()
                    } else {
                        let needed = target_len - s.len();
                        let padding: String = std::iter::repeat(pad_str).flat_map(|s| s.chars()).take(needed).collect();
                        format!("{}{}", padding, s)
                    }
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn rpad(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1), args.get(2)) {
            (Some(Vector::String(v)), Some(Vector::Int64(len)), Some(Vector::String(pad))) => {
                let results_len = v.len().min(len.len()).min(pad.len());
                let results: Vec<String> = (0..results_len).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let target_len = len.get(i).unwrap_or(0) as usize;
                    let pad_str = pad.get(i).unwrap_or(" ");
                    if s.len() >= target_len {
                        s.chars().take(target_len).collect()
                    } else {
                        let needed = target_len - s.len();
                        let padding: String = std::iter::repeat(pad_str).flat_map(|s| s.chars()).take(needed).collect();
                        format!("{}{}", s, padding)
                    }
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn format(&self, args: &[Vector]) -> Vector {
        match (args.first(), args.get(1)) {
            (Some(Vector::Float64(v)), Some(Vector::Int64(decimals))) => {
                let len = v.len().min(decimals.len());
                let results: Vec<String> = (0..len).map(|i| {
                    let n = v.get(i).unwrap_or(0.0);
                    let d = decimals.get(i).unwrap_or(0) as usize;
                    format!("{:.*}", d, n)
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            (Some(Vector::Int64(v)), Some(Vector::Int64(decimals))) => {
                let len = v.len().min(decimals.len());
                let results: Vec<String> = (0..len).map(|i| {
                    let n = v.get(i).unwrap_or(0) as f64;
                    let d = decimals.get(i).unwrap_or(0) as usize;
                    format!("{:.*}", d, n)
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn md5(&self, args: &[Vector]) -> Vector {
        match args.first() {
            Some(Vector::String(v)) => {
                let results: Vec<String> = (0..v.len()).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    format!("{:x}", md5::compute(s))
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }

    fn sha1(&self, args: &[Vector]) -> Vector {
        use sha1::{Sha1, Digest};
        match args.first() {
            Some(Vector::String(v)) => {
                let results: Vec<String> = (0..v.len()).map(|i| {
                    let s = v.get(i).unwrap_or("");
                    let mut hasher = Sha1::new();
                    hasher.update(s.as_bytes());
                    format!("{:x}", hasher.finalize())
                }).collect();
                string_vec(results.iter().map(|s| Some(s.clone())).collect())
            }
            _ => bool_vec(vec![]),
        }
    }
}

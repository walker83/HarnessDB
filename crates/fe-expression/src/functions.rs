use chrono::{NaiveDate, NaiveDateTime, Datelike, Timelike, Duration, Utc, TimeZone};
use types::{ScalarValue, Vector};

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
        .with_ordinal(ordinal.max(1).min(366) as u32)
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
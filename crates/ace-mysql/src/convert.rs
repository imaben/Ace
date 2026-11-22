use ace_core::Value;
use ace_driver::ResultColumn;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use mysql_async::Value as Mv;
use mysql_async::{Column, Row};

pub fn column_meta(col: &Column) -> ResultColumn {
    ResultColumn {
        name: String::from_utf8_lossy(col.name_ref()).into_owned(),
        table: {
            let t = col.table_str();
            if t.is_empty() {
                None
            } else {
                Some(t.into_owned())
            }
        },
        database: {
            let s = col.schema_str();
            if s.is_empty() {
                None
            } else {
                Some(s.into_owned())
            }
        },
        data_type: format!("{:?}", col.column_type()),
        nullable: !col
            .flags()
            .contains(mysql_async::consts::ColumnFlags::NOT_NULL_FLAG),
        primary_key: col
            .flags()
            .contains(mysql_async::consts::ColumnFlags::PRI_KEY_FLAG),
        org_name: {
            let o = col.org_name_str();
            if o.is_empty() {
                None
            } else {
                Some(o.into_owned())
            }
        },
    }
}

pub fn row_to_values(row: Row) -> Vec<Value> {
    let cols = row.columns_ref().len();
    let mut out = Vec::with_capacity(cols);
    for i in 0..cols {
        let v: Option<Mv> = row.get(i);
        out.push(mysql_value_to_ace(v.unwrap_or(Mv::NULL)));
    }
    out
}

pub fn mysql_value_to_ace(v: Mv) -> Value {
    match v {
        Mv::NULL => Value::Null,
        Mv::Bytes(b) => match String::from_utf8(b.clone()) {
            Ok(s) => Value::String(s),
            Err(_) => Value::Bytes(b),
        },
        Mv::Int(i) => Value::I64(i),
        Mv::UInt(u) => Value::U64(u),
        Mv::Float(f) => Value::F64(f as f64),
        Mv::Double(f) => Value::F64(f),
        Mv::Date(y, m, d, h, mi, s, us) => {
            if h == 0 && mi == 0 && s == 0 && us == 0 {
                if let Some(date) = NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32) {
                    return Value::Date(date);
                }
            }
            if let (Some(date), Some(time)) = (
                NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32),
                NaiveTime::from_hms_micro_opt(h as u32, mi as u32, s as u32, us),
            ) {
                Value::DateTime(NaiveDateTime::new(date, time))
            } else {
                Value::String(format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}.{us:06}"))
            }
        }
        Mv::Time(neg, days, h, mi, s, us) => {
            let sign = if neg { "-" } else { "" };
            Value::String(format!(
                "{sign}{}d {h:02}:{mi:02}:{s:02}.{us:06}",
                days
            ))
        }
    }
}
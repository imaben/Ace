use ace_core::{AceError, AceResult, Value};
use ace_driver::result::{QueryResult, ResultColumn};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Sql,
    Csv,
    Xml,
    Html,
    Dot,
    Json,
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub include_structure: bool,
    pub include_data: bool,
    pub drop_table: bool,
    pub gzip: bool,
    pub batch_insert_size: usize,
    pub encoding: String,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Sql,
            include_structure: true,
            include_data: true,
            drop_table: false,
            gzip: false,
            batch_insert_size: 250,
            encoding: "UTF-8".into(),
        }
    }
}

pub fn export_query_result(
    path: &Path,
    result: &QueryResult,
    table: Option<&str>,
    options: &ExportOptions,
) -> AceResult<u64> {
    let file = File::create(path).map_err(|e| AceError::Io(e.to_string()))?;
    let mut writer: Box<dyn Write> = if options.gzip {
        Box::new(BufWriter::new(GzEncoder::new(file, Compression::default())))
    } else {
        Box::new(BufWriter::new(file))
    };

    let bytes = match options.format {
        ExportFormat::Csv => write_csv(&mut writer, result)?,
        ExportFormat::Json => write_json(&mut writer, result)?,
        ExportFormat::Xml => write_xml(&mut writer, result, table.unwrap_or("result"))?,
        ExportFormat::Html => write_html(&mut writer, result, table.unwrap_or("result"))?,
        ExportFormat::Dot => write_dot(&mut writer, table.unwrap_or("result"), &result.columns)?,
        ExportFormat::Sql => write_sql_inserts(
            &mut writer,
            table.unwrap_or("exported_table"),
            result,
            options.batch_insert_size,
        )?,
    };
    writer.flush().map_err(|e| AceError::Io(e.to_string()))?;
    Ok(bytes)
}

pub fn write_csv(w: &mut dyn Write, result: &QueryResult) -> AceResult<u64> {
    let mut wtr = csv::Writer::from_writer(w);
    let headers: Vec<&str> = result.columns.iter().map(|c| c.name.as_str()).collect();
    wtr.write_record(&headers)
        .map_err(|e| AceError::Io(e.to_string()))?;
    for row in &result.rows {
        let rec: Vec<String> = row.iter().map(|v| value_csv(v)).collect();
        wtr.write_record(&rec)
            .map_err(|e| AceError::Io(e.to_string()))?;
    }
    wtr.flush().map_err(|e| AceError::Io(e.to_string()))?;
    Ok(result.rows.len() as u64)
}

fn value_csv(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn write_json(w: &mut dyn Write, result: &QueryResult) -> AceResult<u64> {
    let mut arr = Vec::new();
    for row in &result.rows {
        let mut obj = serde_json::Map::new();
        for (i, col) in result.columns.iter().enumerate() {
            obj.insert(col.name.clone(), value_json(row.get(i).unwrap_or(&Value::Null)));
        }
        arr.push(serde_json::Value::Object(obj));
    }
    serde_json::to_writer_pretty(&mut *w, &arr).map_err(|e| AceError::Io(e.to_string()))?;
    Ok(result.rows.len() as u64)
}

fn value_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::I64(i) => serde_json::json!(i),
        Value::U64(u) => serde_json::json!(u),
        Value::F64(f) => serde_json::json!(f),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => serde_json::Value::String(format!("\\x{}", hex_encode(b))),
        Value::Json(j) => j.clone(),
        other => serde_json::Value::String(other.to_string()),
    }
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn write_xml(w: &mut dyn Write, result: &QueryResult, table: &str) -> AceResult<u64> {
    writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
        .map_err(|e| AceError::Io(e.to_string()))?;
    writeln!(w, "<{}", table).map_err(|e| AceError::Io(e.to_string()))?;
    writeln!(w, ">").map_err(|e| AceError::Io(e.to_string()))?;
    for row in &result.rows {
        writeln!(w, "  <row>").map_err(|e| AceError::Io(e.to_string()))?;
        for (i, col) in result.columns.iter().enumerate() {
            let val = row.get(i).unwrap_or(&Value::Null);
            match val {
                Value::Null => writeln!(w, "    <{n} xsi:nil=\"true\" />", n = xml_escape(&col.name))
                    .map_err(|e| AceError::Io(e.to_string()))?,
                v => writeln!(
                    w,
                    "    <{n}>{}</{n}>",
                    xml_escape(&v.to_string()),
                    n = xml_escape(&col.name)
                )
                .map_err(|e| AceError::Io(e.to_string()))?,
            }
        }
        writeln!(w, "  </row>").map_err(|e| AceError::Io(e.to_string()))?;
    }
    writeln!(w, "</{}>", table).map_err(|e| AceError::Io(e.to_string()))?;
    Ok(result.rows.len() as u64)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn write_html(w: &mut dyn Write, result: &QueryResult, title: &str) -> AceResult<u64> {
    writeln!(
        w,
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title>\
         <style>table{{border-collapse:collapse}}td,th{{border:1px solid #ccc;padding:4px 8px}}\
         th{{background:#f0f0f0}}</style></head><body>",
        xml_escape(title)
    )
    .map_err(|e| AceError::Io(e.to_string()))?;
    writeln!(w, "<h1>{}</h1><table><thead><tr>", xml_escape(title))
        .map_err(|e| AceError::Io(e.to_string()))?;
    for col in &result.columns {
        writeln!(w, "<th>{}</th>", xml_escape(&col.name)).map_err(|e| AceError::Io(e.to_string()))?;
    }
    writeln!(w, "</tr></thead><tbody>").map_err(|e| AceError::Io(e.to_string()))?;
    for row in &result.rows {
        write!(w, "<tr>").map_err(|e| AceError::Io(e.to_string()))?;
        for (i, _) in result.columns.iter().enumerate() {
            let v = row.get(i).unwrap_or(&Value::Null);
            write!(w, "<td>{}</td>", xml_escape(&v.to_string()))
                .map_err(|e| AceError::Io(e.to_string()))?;
        }
        writeln!(w, "</tr>").map_err(|e| AceError::Io(e.to_string()))?;
    }
    writeln!(w, "</tbody></table></body></html>").map_err(|e| AceError::Io(e.to_string()))?;
    Ok(result.rows.len() as u64)
}

pub fn write_dot(w: &mut dyn Write, table: &str, columns: &[ResultColumn]) -> AceResult<u64> {
    writeln!(w, "digraph {table} {{").map_err(|e| AceError::Io(e.to_string()))?;
    writeln!(w, "  node [shape=record];").map_err(|e| AceError::Io(e.to_string()))?;
    let fields: Vec<String> = columns
        .iter()
        .map(|c| format!("<{}> {}", c.name, c.name))
        .collect();
    writeln!(w, "  {table} [label=\"{{{table}|{}}}\"];", fields.join("|"))
        .map_err(|e| AceError::Io(e.to_string()))?;
    writeln!(w, "}}").map_err(|e| AceError::Io(e.to_string()))?;
    Ok(1)
}

pub fn write_sql_inserts(
    w: &mut dyn Write,
    table: &str,
    result: &QueryResult,
    batch: usize,
) -> AceResult<u64> {
    if result.columns.is_empty() {
        return Ok(0);
    }
    let cols: Vec<String> = result
        .columns
        .iter()
        .map(|c| format!("`{}`", c.name.replace('`', "``")))
        .collect();
    let col_list = cols.join(", ");
    let batch = batch.max(1);
    let mut written = 0u64;

    for chunk in result.rows.chunks(batch) {
        writeln!(w, "INSERT INTO `{table}` ({col_list}) VALUES").map_err(|e| AceError::Io(e.to_string()))?;
        for (i, row) in chunk.iter().enumerate() {
            let vals: Vec<String> = row.iter().map(|v| v.to_sql_literal()).collect();
            let sep = if i + 1 == chunk.len() { ";" } else { "," };
            writeln!(w, "  ({}){sep}", vals.join(", ")).map_err(|e| AceError::Io(e.to_string()))?;
            written += 1;
        }
        writeln!(w).map_err(|e| AceError::Io(e.to_string()))?;
    }
    Ok(written)
}

pub fn write_create_table_sql(w: &mut dyn Write, create_sql: &str, drop_first: bool, table: &str) -> AceResult<()> {
    if drop_first {
        writeln!(w, "DROP TABLE IF EXISTS `{table}`;").map_err(|e| AceError::Io(e.to_string()))?;
    }
    writeln!(w, "{create_sql};").map_err(|e| AceError::Io(e.to_string()))?;
    writeln!(w).map_err(|e| AceError::Io(e.to_string()))?;
    Ok(())
}
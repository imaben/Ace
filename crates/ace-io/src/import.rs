use ace_core::{AceError, AceResult};
use ace_sql::split::StatementLexer;
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvFieldMapping {
    pub source_index: usize,
    pub source_name: String,
    pub target_column: Option<String>,
    pub skip: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CsvImportMode {
    Insert,
    Replace,
    Update,
    InsertIgnore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvImportOptions {
    pub delimiter: u8,
    pub has_headers: bool,
    pub mode: CsvImportMode,
    pub encoding: String,
    pub create_table: bool,
    pub table_name: String,
    pub mappings: Vec<CsvFieldMapping>,
}

impl Default for CsvImportOptions {
    fn default() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
            mode: CsvImportMode::Insert,
            encoding: "UTF-8".into(),
            create_table: false,
            table_name: "imported".into(),
            mappings: Vec::new(),
        }
    }
}

pub struct ImportProgress {
    pub statements_done: u64,
    pub rows_done: u64,
    pub errors: Vec<String>,
}

/// Stream a SQL file into individual statements.
pub fn stream_sql_file(path: &Path, encoding_label: &str) -> AceResult<Vec<String>> {
    let mut file = File::open(path).map_err(|e| AceError::Io(e.to_string()))?;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw)
        .map_err(|e| AceError::Io(e.to_string()))?;

    let encoding = Encoding::for_label(encoding_label.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    let (cow, _, _) = encoding.decode(&raw);

    let mut lexer = StatementLexer::new();
    let mut stmts = lexer.push(&cow);
    if let Some(last) = lexer.finish() {
        stmts.push(last);
    }
    Ok(stmts)
}

/// Preview CSV headers and first rows for the field mapper UI.
pub fn preview_csv(
    path: &Path,
    delimiter: u8,
    has_headers: bool,
    max_rows: usize,
) -> AceResult<(Vec<String>, Vec<Vec<String>>)> {
    let file = File::open(path).map_err(|e| AceError::Io(e.to_string()))?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(has_headers)
        .from_reader(BufReader::new(file));

    let headers: Vec<String> = if has_headers {
        rdr.headers()
            .map_err(|e| AceError::Io(e.to_string()))?
            .iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut rows: Vec<Vec<String>> = Vec::new();
    for (i, rec) in rdr.records().enumerate() {
        if i >= max_rows {
            break;
        }
        let rec = rec.map_err(|e| AceError::Io(e.to_string()))?;
        rows.push(rec.iter().map(|s| s.to_string()).collect());
    }

    let headers = if headers.is_empty() {
        let width = rows.first().map(|r| r.len()).unwrap_or(0);
        (0..width).map(|i| format!("column_{i}")).collect()
    } else {
        headers
    };

    Ok((headers, rows))
}

pub fn csv_rows_to_insert_sql(
    options: &CsvImportOptions,
    rows: &[Vec<String>],
) -> AceResult<Vec<String>> {
    let cols: Vec<&str> = options
        .mappings
        .iter()
        .filter(|m| !m.skip && m.target_column.is_some())
        .map(|m| m.target_column.as_deref().unwrap())
        .collect();
    if cols.is_empty() {
        return Err(AceError::msg("no target columns mapped"));
    }
    let col_list = cols
        .iter()
        .map(|c| format!("`{}`", c.replace('`', "``")))
        .collect::<Vec<_>>()
        .join(", ");

    let verb = match options.mode {
        CsvImportMode::Insert => "INSERT",
        CsvImportMode::Replace => "REPLACE",
        CsvImportMode::InsertIgnore => "INSERT IGNORE",
        CsvImportMode::Update => "INSERT", // UPDATE via ON DUPLICATE handled by caller
    };

    let mut out = Vec::new();
    for row in rows {
        let vals: Vec<String> = options
            .mappings
            .iter()
            .filter(|m| !m.skip && m.target_column.is_some())
            .map(|m| {
                let raw = row.get(m.source_index).map(|s| s.as_str()).unwrap_or("");
                if raw.is_empty() {
                    "NULL".into()
                } else {
                    format!("'{}'", raw.replace('\\', "\\\\").replace('\'', "''"))
                }
            })
            .collect();
        out.push(format!(
            "{verb} INTO `{}` ({col_list}) VALUES ({})",
            options.table_name,
            vals.join(", ")
        ));
    }
    Ok(out)
}

/// Detect if a file looks like SQL vs CSV by extension / first line.
pub fn detect_import_kind(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "sql" => "sql",
        "csv" | "tsv" | "txt" => "csv",
        "xml" => "xml",
        "json" => "json",
        _ => {
            if let Ok(file) = File::open(path) {
                let mut line = String::new();
                if BufReader::new(file).read_line(&mut line).is_ok() {
                    let t = line.trim_start().to_uppercase();
                    if t.starts_with("INSERT")
                        || t.starts_with("CREATE")
                        || t.starts_with("DROP")
                        || t.starts_with("--")
                    {
                        return "sql";
                    }
                }
            }
            "csv"
        }
    }
}
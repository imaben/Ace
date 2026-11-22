use crate::convert::{mysql_value_to_ace, row_to_values};
use ace_core::{AceError, AceResult, Value};
use ace_driver::meta::*;
use mysql_async::prelude::*;
use mysql_async::{Conn, Row};

pub async fn fetch_server_info(conn: &mut Conn, host: &str, port: u16) -> AceResult<ServerInfo> {
    let row: Option<Row> = conn
        .query_first(
            "SELECT VERSION(), @@version_comment, CURRENT_USER(), DATABASE(), @@character_set_connection",
        )
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;
    let row = row.ok_or_else(|| AceError::Query("empty server info".into()))?;
    let vals = row_to_values(row);
    Ok(ServerInfo {
        version: val_str(&vals, 0),
        version_comment: val_str(&vals, 1),
        host: host.to_string(),
        port,
        current_user: val_str(&vals, 2),
        current_database: match &vals.get(3) {
            Some(Value::Null) | None => None,
            Some(v) => Some(v.to_string()),
        },
        charset: val_str(&vals, 4),
        engine_name: if val_str(&vals, 0).to_lowercase().contains("mariadb") {
            "MariaDB".into()
        } else {
            "MySQL".into()
        },
    })
}

pub async fn list_databases(conn: &mut Conn) -> AceResult<Vec<String>> {
    let rows: Vec<Row> = conn
        .query("SHOW DATABASES")
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let vals = row_to_values(r);
            val_str(&vals, 0)
        })
        .collect())
}

pub async fn list_objects(conn: &mut Conn, database: &str) -> AceResult<Vec<DatabaseObject>> {
    let sql = format!(
        "SELECT TABLE_NAME, TABLE_TYPE, ENGINE, TABLE_ROWS, DATA_LENGTH, TABLE_COMMENT \
         FROM information_schema.TABLES WHERE TABLE_SCHEMA = '{}' ORDER BY TABLE_NAME",
        escape(database)
    );
    let rows: Vec<Row> = conn
        .query(sql)
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;
    let mut out = Vec::new();
    for r in rows {
        let v = row_to_values(r);
        let table_type = val_str(&v, 1);
        let kind = if table_type.contains("VIEW") {
            ObjectKind::View
        } else {
            ObjectKind::Table
        };
        out.push(DatabaseObject {
            name: val_str(&v, 0),
            kind,
            engine: opt_str(&v, 2),
            rows_estimate: opt_u64(&v, 3),
            data_length: opt_u64(&v, 4),
            comment: opt_str(&v, 5),
            pinned: false,
        });
    }

    // Routines
    let sql = format!(
        "SELECT ROUTINE_NAME, ROUTINE_TYPE FROM information_schema.ROUTINES \
         WHERE ROUTINE_SCHEMA = '{}' ORDER BY ROUTINE_NAME",
        escape(database)
    );
    if let Ok(rows) = conn.query::<Row, _>(sql).await {
        for r in rows {
            let v = row_to_values(r);
            let kind = if val_str(&v, 1).eq_ignore_ascii_case("PROCEDURE") {
                ObjectKind::Procedure
            } else {
                ObjectKind::Function
            };
            out.push(DatabaseObject {
                name: val_str(&v, 0),
                kind,
                engine: None,
                rows_estimate: None,
                data_length: None,
                comment: None,
                pinned: false,
            });
        }
    }
    Ok(out)
}

pub async fn table_meta(conn: &mut Conn, database: &str, table: &str) -> AceResult<TableMeta> {
    let columns = list_columns(conn, database, table).await?;
    let indexes = list_indexes(conn, database, table).await?;
    let foreign_keys = list_foreign_keys(conn, database, table).await?;

    let create_sql: Option<String> = {
        let sql = format!(
            "SHOW CREATE TABLE `{}`.`{}`",
            escape_ident(database),
            escape_ident(table)
        );
        match conn.query_first::<Row, _>(sql).await {
            Ok(Some(row)) => {
                let v = row_to_values(row);
                Some(val_str(&v, 1))
            }
            _ => None,
        }
    };

    let status: Option<Row> = conn
        .query_first(format!(
            "SHOW TABLE STATUS FROM `{}` WHERE Name = '{}'",
            escape_ident(database),
            escape(table)
        ))
        .await
        .unwrap_or(None);

    let (engine, charset, collation, comment, auto_inc, create_options) = if let Some(row) = status
    {
        let v = row_to_values(row);
        // 0:Name, 1:Engine, 2:Version, 3:Row_format, 4:Rows, 5:Avg_row_length, 6:Data_length,
        // 7:Max_data_length, 8:Index_length, 9:Data_free, 10:Auto_increment, 11:Create_time,
        // 12:Update_time, 13:Check_time, 14:Collation, 15:Checksum, 16:Create_options, 17:Comment
        let collation = opt_str(&v, 14);
        let charset = collation
            .as_deref()
            .and_then(|c| c.split('_').next())
            .map(|s| s.to_string());
        (
            opt_str(&v, 1),
            charset,
            collation,
            opt_str(&v, 17),
            opt_u64(&v, 10),
            opt_str(&v, 16),
        )
    } else {
        (None, None, None, None, None, None)
    };

    Ok(TableMeta {
        name: table.to_string(),
        database: database.to_string(),
        engine,
        charset,
        collation,
        comment,
        auto_increment: auto_inc,
        create_options,
        columns,
        indexes,
        foreign_keys,
        create_sql,
    })
}

async fn list_columns(conn: &mut Conn, database: &str, table: &str) -> AceResult<Vec<ColumnMeta>> {
    let sql = format!(
        "SELECT COLUMN_NAME, DATA_TYPE, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY, EXTRA, \
         COLUMN_DEFAULT, COLUMN_COMMENT, CHARACTER_SET_NAME, COLLATION_NAME, \
         CHARACTER_MAXIMUM_LENGTH, NUMERIC_PRECISION, NUMERIC_SCALE, ORDINAL_POSITION \
         FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY ORDINAL_POSITION",
        escape(database),
        escape(table)
    );
    let rows: Vec<Row> = conn
        .query(sql)
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let v = row_to_values(r);
            let full_type = val_str(&v, 2);
            ColumnMeta {
                name: val_str(&v, 0),
                data_type: val_str(&v, 1),
                full_type: full_type.clone(),
                nullable: val_str(&v, 3).eq_ignore_ascii_case("YES"),
                primary_key: val_str(&v, 4).contains("PRI"),
                auto_increment: val_str(&v, 5).to_lowercase().contains("auto_increment"),
                default: opt_str(&v, 6),
                extra: opt_str(&v, 5),
                comment: opt_str(&v, 7),
                charset: opt_str(&v, 8),
                collation: opt_str(&v, 9),
                unsigned: full_type.to_lowercase().contains("unsigned"),
                zerofill: full_type.to_lowercase().contains("zerofill"),
                character_maximum_length: opt_u64(&v, 10),
                numeric_precision: opt_u64(&v, 11).map(|x| x as u32),
                numeric_scale: opt_u64(&v, 12).map(|x| x as u32),
                ordinal_position: opt_u64(&v, 13).unwrap_or(0) as u32,
            }
        })
        .collect())
}

pub async fn list_indexes(conn: &mut Conn, database: &str, table: &str) -> AceResult<Vec<IndexMeta>> {
    let sql = format!(
        "SHOW INDEX FROM `{}`.`{}`",
        escape_ident(database),
        escape_ident(table)
    );
    let rows: Vec<Row> = conn
        .query(sql)
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;

    let mut map: Vec<IndexMeta> = Vec::new();
    for r in rows {
        let v = row_to_values(r);
        let name = val_str(&v, 2);
        let is_unique = match v.get(1) {
            Some(Value::I64(0)) | Some(Value::U64(0)) => true,
            Some(Value::String(s)) if s == "0" => true,
            _ => false,
        };
        let column = val_str(&v, 4);
        let sub_part = opt_u64(&v, 7).map(|n| n as u32);
        let index_type = val_str(&v, 10);
        let comment = opt_str(&v, 12).or_else(|| opt_str(&v, 11));
        if let Some(existing) = map.iter_mut().find(|i| i.name == name) {
            existing.columns.push(IndexColumn {
                name: column,
                sub_part,
                descending: false,
            });
        } else {
            map.push(IndexMeta {
                name: name.clone(),
                columns: vec![IndexColumn {
                    name: column,
                    sub_part,
                    descending: false,
                }],
                unique: is_unique || name == "PRIMARY",
                primary: name == "PRIMARY",
                index_type,
                comment,
            });
        }
    }
    Ok(map)
}

pub async fn list_foreign_keys(
    conn: &mut Conn,
    database: &str,
    table: &str,
) -> AceResult<Vec<ForeignKeyMeta>> {
    let sql = format!(
        "SELECT kcu.CONSTRAINT_NAME, kcu.COLUMN_NAME, kcu.REFERENCED_TABLE_SCHEMA, \
         kcu.REFERENCED_TABLE_NAME, kcu.REFERENCED_COLUMN_NAME, \
         COALESCE(rc.UPDATE_RULE, 'RESTRICT'), COALESCE(rc.DELETE_RULE, 'RESTRICT') \
         FROM information_schema.KEY_COLUMN_USAGE kcu \
         LEFT JOIN information_schema.REFERENTIAL_CONSTRAINTS rc \
           ON rc.CONSTRAINT_SCHEMA = kcu.TABLE_SCHEMA \
          AND rc.TABLE_NAME = kcu.TABLE_NAME \
          AND rc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME \
         WHERE kcu.TABLE_SCHEMA = '{}' AND kcu.TABLE_NAME = '{}' \
           AND kcu.REFERENCED_TABLE_NAME IS NOT NULL \
         ORDER BY kcu.CONSTRAINT_NAME, kcu.ORDINAL_POSITION",
        escape(database),
        escape(table)
    );
    let rows: Vec<Row> = conn.query(sql).await.unwrap_or_default();
    let mut out: Vec<ForeignKeyMeta> = Vec::new();
    for r in rows {
        let v = row_to_values(r);
        let name = val_str(&v, 0);
        if let Some(fk) = out.iter_mut().find(|f| f.name == name) {
            fk.columns.push(val_str(&v, 1));
            fk.ref_columns.push(val_str(&v, 4));
        } else {
            out.push(ForeignKeyMeta {
                name,
                columns: vec![val_str(&v, 1)],
                ref_database: opt_str(&v, 2),
                ref_table: val_str(&v, 3),
                ref_columns: vec![val_str(&v, 4)],
                on_update: opt_str(&v, 5).unwrap_or_else(|| "RESTRICT".into()),
                on_delete: opt_str(&v, 6).unwrap_or_else(|| "RESTRICT".into()),
            });
        }
    }
    Ok(out)
}

pub async fn list_triggers(
    conn: &mut Conn,
    database: &str,
    table: &str,
) -> AceResult<Vec<TriggerMeta>> {
    let sql = format!(
        "SHOW TRIGGERS FROM `{}` WHERE `Table` = '{}'",
        escape_ident(database),
        escape(table)
    );
    let rows: Vec<Row> = conn.query(sql).await.unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|r| {
            let v = row_to_values(r);
            TriggerMeta {
                name: val_str(&v, 0),
                event: val_str(&v, 1),
                // table at 2
                statement: val_str(&v, 3),
                timing: val_str(&v, 4),
                definer: opt_str(&v, 7),
            }
        })
        .collect())
}

pub async fn process_list(conn: &mut Conn) -> AceResult<Vec<ProcessInfo>> {
    let rows: Vec<Row> = conn
        .query("SHOW FULL PROCESSLIST")
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let v = row_to_values(r);
            ProcessInfo {
                id: opt_u64(&v, 0).unwrap_or(0),
                user: val_str(&v, 1),
                host: val_str(&v, 2),
                database: opt_str(&v, 3),
                command: val_str(&v, 4),
                time_secs: opt_u64(&v, 5).unwrap_or(0),
                state: opt_str(&v, 6),
                info: opt_str(&v, 7),
            }
        })
        .collect())
}

pub async fn server_variables(conn: &mut Conn) -> AceResult<std::collections::HashMap<String, String>> {
    let rows: Vec<Row> = conn
        .query("SHOW VARIABLES")
        .await
        .map_err(|e| AceError::Query(e.to_string()))?;
    let mut map = std::collections::HashMap::new();
    for r in rows {
        let v = row_to_values(r);
        map.insert(val_str(&v, 0), val_str(&v, 1));
    }
    Ok(map)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

fn escape_ident(s: &str) -> String {
    s.replace('`', "``")
}

fn val_str(vals: &[Value], i: usize) -> String {
    match vals.get(i) {
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}

fn opt_str(vals: &[Value], i: usize) -> Option<String> {
    match vals.get(i) {
        Some(Value::Null) | None => None,
        Some(v) => {
            let s = v.to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
    }
}

fn opt_u64(vals: &[Value], i: usize) -> Option<u64> {
    match vals.get(i) {
        Some(Value::I64(v)) => Some(*v as u64),
        Some(Value::U64(v)) => Some(*v),
        Some(Value::String(s)) => s.parse().ok(),
        Some(Value::F64(f)) => Some(*f as u64),
        _ => None,
    }
}

#[allow(dead_code)]
pub fn _use_mysql_value(v: mysql_async::Value) -> Value {
    mysql_value_to_ace(v)
}
//! DDL helpers for structure editing (MySQL-oriented; dialects can specialize).

use ace_driver::meta::ColumnMeta;

pub fn add_column_sql(database: &str, table: &str, col: &ColumnMeta) -> String {
    format!(
        "ALTER TABLE `{}`.`{}` ADD COLUMN {}",
        esc(database),
        esc(table),
        column_def(col)
    )
}

pub fn modify_column_sql(database: &str, table: &str, col: &ColumnMeta) -> String {
    format!(
        "ALTER TABLE `{}`.`{}` MODIFY COLUMN {}",
        esc(database),
        esc(table),
        column_def(col)
    )
}

pub fn drop_column_sql(database: &str, table: &str, column: &str) -> String {
    format!(
        "ALTER TABLE `{}`.`{}` DROP COLUMN `{}`",
        esc(database),
        esc(table),
        esc(column)
    )
}

pub fn add_index_sql(
    database: &str,
    table: &str,
    name: &str,
    columns: &[&str],
    unique: bool,
) -> String {
    let cols = columns
        .iter()
        .map(|c| format!("`{}`", esc(c)))
        .collect::<Vec<_>>()
        .join(", ");
    let kind = if unique { "UNIQUE INDEX" } else { "INDEX" };
    format!(
        "ALTER TABLE `{}`.`{}` ADD {kind} `{}` ({cols})",
        esc(database),
        esc(table),
        esc(name)
    )
}

pub fn drop_index_sql(database: &str, table: &str, name: &str) -> String {
    if name == "PRIMARY" {
        format!(
            "ALTER TABLE `{}`.`{}` DROP PRIMARY KEY",
            esc(database),
            esc(table)
        )
    } else {
        format!(
            "ALTER TABLE `{}`.`{}` DROP INDEX `{}`",
            esc(database),
            esc(table),
            esc(name)
        )
    }
}

pub fn add_foreign_key_sql(
    database: &str,
    table: &str,
    name: &str,
    columns: &[&str],
    ref_table: &str,
    ref_columns: &[&str],
    on_update: &str,
    on_delete: &str,
) -> String {
    let cols = columns.iter().map(|c| format!("`{}`", esc(c))).collect::<Vec<_>>().join(", ");
    let refs = ref_columns.iter().map(|c| format!("`{}`", esc(c))).collect::<Vec<_>>().join(", ");
    format!(
        "ALTER TABLE `{}`.`{}` ADD CONSTRAINT `{}` FOREIGN KEY ({cols}) \
         REFERENCES `{}` ({refs}) ON UPDATE {on_update} ON DELETE {on_delete}",
        esc(database),
        esc(table),
        esc(name),
        esc(ref_table)
    )
}

pub fn create_trigger_sql(
    database: &str,
    name: &str,
    timing: &str,
    event: &str,
    table: &str,
    body: &str,
) -> String {
    format!(
        "CREATE TRIGGER `{}`.`{}` {timing} {event} ON `{}`.`{}` FOR EACH ROW {body}",
        esc(database),
        esc(name),
        esc(database),
        esc(table)
    )
}

pub fn rename_table_sql(database: &str, from: &str, to: &str) -> String {
    format!(
        "RENAME TABLE `{}`.`{}` TO `{}`.`{}`",
        esc(database),
        esc(from),
        esc(database),
        esc(to)
    )
}

pub fn create_database_sql(name: &str, charset: Option<&str>, collation: Option<&str>) -> String {
    let mut sql = format!("CREATE DATABASE `{}`", esc(name));
    if let Some(cs) = charset.filter(|s| !s.trim().is_empty()) {
        sql.push_str(&format!(" CHARACTER SET {}", cs.trim()));
    }
    if let Some(col) = collation.filter(|s| !s.trim().is_empty()) {
        sql.push_str(&format!(" COLLATE {}", col.trim()));
    }
    sql
}

pub fn drop_database_sql(name: &str) -> String {
    format!("DROP DATABASE `{}`", esc(name))
}

pub fn create_table_sql(
    database: &str,
    table: &str,
    engine: Option<&str>,
    charset: Option<&str>,
    is_sqlite: bool,
) -> String {
    if is_sqlite {
        return format!(
            "CREATE TABLE \"{}\" (\n  \"id\" INTEGER PRIMARY KEY AUTOINCREMENT\n)",
            table.replace('"', "\"\"")
        );
    }
    let mut sql = format!(
        "CREATE TABLE `{}`.`{}` (\n  `id` BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,\n  PRIMARY KEY (`id`)\n)",
        esc(database),
        esc(table)
    );
    if let Some(eng) = engine.filter(|s| !s.trim().is_empty()) {
        sql.push_str(&format!(" ENGINE={}", eng.trim()));
    }
    if let Some(cs) = charset.filter(|s| !s.trim().is_empty()) {
        sql.push_str(&format!(" DEFAULT CHARSET={}", cs.trim()));
    }
    sql
}

pub fn drop_table_sql(database: &str, table: &str, is_sqlite: bool) -> String {
    if is_sqlite {
        format!("DROP TABLE \"{}\"", table.replace('"', "\"\""))
    } else {
        format!("DROP TABLE `{}`.`{}`", esc(database), esc(table))
    }
}

pub fn truncate_table_sql(database: &str, table: &str, is_sqlite: bool) -> String {
    if is_sqlite {
        format!("DELETE FROM \"{}\"", table.replace('"', "\"\""))
    } else {
        format!("TRUNCATE TABLE `{}`.`{}`", esc(database), esc(table))
    }
}

pub fn drop_foreign_key_sql(database: &str, table: &str, constraint: &str) -> String {
    format!(
        "ALTER TABLE `{}`.`{}` DROP FOREIGN KEY `{}`",
        esc(database),
        esc(table),
        esc(constraint)
    )
}

pub fn drop_trigger_sql(database: &str, trigger: &str, is_sqlite: bool) -> String {
    if is_sqlite {
        format!("DROP TRIGGER \"{}\"", trigger.replace('"', "\"\""))
    } else {
        format!("DROP TRIGGER `{}`.`{}`", esc(database), esc(trigger))
    }
}

pub fn alter_table_comment_sql(database: &str, table: &str, comment: &str) -> String {
    format!(
        "ALTER TABLE `{}`.`{}` COMMENT = '{}'",
        esc(database),
        esc(table),
        comment.replace('\\', "\\\\").replace('\'', "''")
    )
}

pub fn change_column_sql(database: &str, table: &str, old_name: &str, col: &ColumnMeta) -> String {
    if old_name == col.name {
        modify_column_sql(database, table, col)
    } else {
        format!(
            "ALTER TABLE `{}`.`{}` CHANGE COLUMN `{}` {}",
            esc(database),
            esc(table),
            esc(old_name),
            column_def(col)
        )
    }
}

fn column_def(col: &ColumnMeta) -> String {
    let mut s = format!("`{}` {}", esc(&col.name), col.full_type);
    if !col.nullable {
        s.push_str(" NOT NULL");
    }
    if let Some(default) = &col.default {
        if default.eq_ignore_ascii_case("CURRENT_TIMESTAMP") {
            s.push_str(" DEFAULT CURRENT_TIMESTAMP");
        } else {
            s.push_str(&format!(" DEFAULT '{default}'"));
        }
    }
    if col.auto_increment {
        s.push_str(" AUTO_INCREMENT");
    }
    if let Some(comment) = &col.comment {
        s.push_str(&format!(" COMMENT '{}'", comment.replace('\'', "''")));
    }
    s
}

fn esc(s: &str) -> String {
    s.replace('`', "``")
}
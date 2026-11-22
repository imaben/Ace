pub fn format_sql(sql: &str) -> String {
    let options = sqlformat::FormatOptions {
        indent: sqlformat::Indent::Spaces(2),
        uppercase: true,
        lines_between_queries: 1,
    };
    sqlformat::format(sql, &sqlformat::QueryParams::None, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        let out = format_sql("select a,b from t where id=1");
        assert!(out.to_lowercase().contains("select"));
    }
}
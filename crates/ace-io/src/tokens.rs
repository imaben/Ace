use chrono::Local;

/// Expand filename tokens such as `$DATABASE`, `$TABLE`, `$DATE`, `$TIME`.
pub fn expand_filename_tokens(template: &str, database: &str, table: &str) -> String {
    let now = Local::now();
    template
        .replace("$DATABASE", database)
        .replace("$TABLE", table)
        .replace("$DATE", &now.format("%Y-%m-%d").to_string())
        .replace("$TIME", &now.format("%H%M%S").to_string())
        .replace("$DATETIME", &now.format("%Y-%m-%d_%H%M%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands() {
        let s = expand_filename_tokens("$DATABASE-$TABLE.sql", "app", "users");
        assert_eq!(s, "app-users.sql");
    }
}
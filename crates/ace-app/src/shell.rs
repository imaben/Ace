use crate::runtime::{open_url, registry};
use ace_core::parse_connection_string;
use ace_driver::meta::ObjectKind;
use ace_driver::query::{CancelToken, QueryOptions};
use ace_io::export::{export_query_result, ExportFormat, ExportOptions};
use ace_sql::{complete, format_sql, split_statements, word_at_cursor, CompletionContext};
use ace_store::{load_favorites, load_settings, save_settings, HistoryStore};
use ace_ui::{AppState, AppScreen, MainViewTab};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

pub async fn run_shell(url: &str) -> anyhow::Result<()> {
    let info = parse_connection_string(url).map_err(|e| anyhow::anyhow!(e))?;
    let conn = open_url(url).await?;
    let settings = load_settings().unwrap_or_default();
    let history = HistoryStore::open().ok();

    let mut app = AppState::default();
    app.settings = settings.clone();
    app.preferences.settings = settings;
    if let Ok(fav) = load_favorites() {
        app.favorites.tree = fav;
    }
    app.open_session(info.clone());

    let server = conn.server_info().await?;
    {
        let session = app.active_session_mut().unwrap();
        session.status = format!(
            "{} {} @ {}:{}",
            server.engine_name, server.version, server.host, server.port
        );
        session.databases = conn.list_databases().await.unwrap_or_default();
        let db = if info.engine == ace_core::EngineKind::Sqlite {
            "main".to_string()
        } else {
            info.database
                .clone()
                .or(server.current_database.clone())
                .unwrap_or_default()
        };
        if !db.is_empty() {
            session.current_database = Some(db.clone());
            session.objects = conn.list_objects(&db).await.unwrap_or_default();
        }
    }

    println!("Ace interactive shell");
    println!("{}", app.active_session().unwrap().status);
    println!("Commands: .help  .db <name>  .tables  .use <table>  .structure  .content");
    println!("          .query / SQL  .explain <sql>  .procs  .vars  .export <file> <sql>");
    println!("          .format  .history  .tab structure|content|query  .quit");

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        let session = app.active_session().unwrap();
        let prompt = format!(
            "ace:{}:{}> ",
            session.current_database.as_deref().unwrap_or("-"),
            session.selected_table.as_deref().unwrap_or("-")
        );
        write!(stdout, "{prompt}")?;
        stdout.flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line == ".quit" || line == ".exit" {
            break;
        }
        if line == ".help" {
            print_help();
            continue;
        }
        if line == ".info" {
            println!("{}", app.active_session().unwrap().status);
            continue;
        }
        if let Some(name) = line.strip_prefix(".db ") {
            let name = name.trim();
            conn.use_database(name).await?;
            let objects = conn.list_objects(name).await?;
            let session = app.active_session_mut().unwrap();
            session.current_database = Some(name.to_string());
            session.objects = objects;
            session.selected_table = None;
            println!("using database {name}");
            continue;
        }
        if line == ".tables" || line == ".objects" {
            let session = app.active_session().unwrap();
            for o in session.filtered_objects() {
                let kind = match o.kind {
                    ObjectKind::Table => "T",
                    ObjectKind::View => "V",
                    ObjectKind::Procedure => "P",
                    ObjectKind::Function => "F",
                    ObjectKind::Trigger => "R",
                    ObjectKind::Event => "E",
                };
                println!("  [{kind}] {}", o.name);
            }
            continue;
        }
        if let Some(table) = line.strip_prefix(".use ") {
            let table = table.trim().to_string();
            let db = app
                .active_session()
                .unwrap()
                .current_database
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no database selected"))?;
            load_table(&conn, &mut app, &db, &table).await?;
            continue;
        }
        if line == ".structure" {
            app.active_session_mut().unwrap().main_tab = MainViewTab::Structure;
            if let Some(meta) = &app.active_session().unwrap().structure.meta {
                for c in &meta.columns {
                    println!(
                        "  {} {} {} {}",
                        c.name,
                        c.full_type,
                        if c.nullable { "NULL" } else { "NOT NULL" },
                        if c.primary_key { "PK" } else { "" }
                    );
                }
            } else {
                println!("select a table with .use <table> first");
            }
            continue;
        }
        if line == ".content" {
            app.active_session_mut().unwrap().main_tab = MainViewTab::Content;
            print_content(app.active_session().unwrap());
            continue;
        }
        if line == ".relations" {
            for fk in &app.active_session().unwrap().relations.foreign_keys {
                println!(
                    "  {} ({}) -> {}.{} ({}) ON UPDATE {} ON DELETE {}",
                    fk.name,
                    fk.columns.join(","),
                    fk.ref_table,
                    fk.ref_columns.join(","),
                    fk.ref_database.as_deref().unwrap_or(""),
                    fk.on_update,
                    fk.on_delete
                );
            }
            continue;
        }
        if line == ".triggers" {
            for t in &app.active_session().unwrap().triggers.triggers {
                println!("  {} {} {} : {}", t.name, t.timing, t.event, t.statement);
            }
            continue;
        }
        if line == ".info-table" || line == ".tableinfo" {
            let ti = &app.active_session().unwrap().table_info;
            println!("engine={:?} charset={:?} rows={:?} comment={:?}", ti.engine, ti.charset, ti.row_count, ti.comment);
            if let Some(sql) = &ti.create_sql {
                println!("{sql}");
            }
            continue;
        }
        if line == ".procs" {
            let list = conn.process_list().await?;
            app.active_session_mut().unwrap().processes.processes = list.clone();
            for p in list {
                println!(
                    "  {:>5} {}@{} [{}] {}s {}",
                    p.id,
                    p.user,
                    p.host,
                    p.command,
                    p.time_secs,
                    p.info.as_deref().unwrap_or("")
                );
            }
            continue;
        }
        if line == ".vars" {
            let vars = conn.server_variables().await?;
            app.active_session_mut().unwrap().variables.variables = vars.clone();
            let mut keys: Vec<_> = vars.keys().collect();
            keys.sort();
            for k in keys.into_iter().take(40) {
                println!("  {k} = {}", vars.get(k).unwrap());
            }
            continue;
        }
        if line == ".users" {
            match conn.list_users().await {
                Ok(users) => {
                    for u in users {
                        println!("  '{}'@'{}'", u.user, u.host);
                    }
                }
                Err(e) => println!("error: {e}"),
            }
            continue;
        }
        if line == ".history" {
            if let Some(h) = &history {
                for e in h.recent(20)? {
                    let mark = if e.success { "ok" } else { "ERR" };
                    println!("  [{mark}] {}ms  {}", e.duration_ms, e.sql.replace('\n', " "));
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix(".export ") {
            let mut parts = rest.splitn(2, ' ');
            let file = parts.next().unwrap_or("");
            let sql = parts.next().unwrap_or("");
            if file.is_empty() || sql.is_empty() {
                println!("usage: .export <file.csv> <sql>");
                continue;
            }
            let result = conn
                .execute(sql, QueryOptions::default(), CancelToken::new())
                .await?;
            let n = export_query_result(
                Path::new(file),
                &result,
                app.active_session().and_then(|s| s.selected_table.as_deref()),
                &ExportOptions {
                    format: ExportFormat::Csv,
                    ..Default::default()
                },
            )?;
            println!("wrote {n} rows to {file}");
            continue;
        }
        if let Some(sql) = line.strip_prefix(".explain ") {
            let result = conn
                .execute(
                    &format!("EXPLAIN {sql}"),
                    QueryOptions::default(),
                    CancelToken::new(),
                )
                .await?;
            print_result(&result);
            continue;
        }
        if let Some(tab) = line.strip_prefix(".tab ") {
            let tab = match tab.trim() {
                "structure" => MainViewTab::Structure,
                "content" => MainViewTab::Content,
                "query" => MainViewTab::CustomQuery,
                "relations" => MainViewTab::Relations,
                "triggers" => MainViewTab::Triggers,
                "info" => MainViewTab::TableInfo,
                _ => {
                    println!("unknown tab");
                    continue;
                }
            };
            app.active_session_mut().unwrap().main_tab = tab;
            println!("tab -> {}", tab.label());
            continue;
        }
        if line == ".format" {
            let sql = app
                .active_session()
                .unwrap()
                .query
                .active()
                .map(|t| t.sql.clone())
                .unwrap_or_default();
            if sql.is_empty() {
                println!("no query buffer; type SQL first");
            } else {
                println!("{}", format_sql(&sql));
            }
            continue;
        }
        if line == ".screen connection" {
            app.screen = AppScreen::Connection;
            println!("screen -> connection");
            continue;
        }
        if line.starts_with('.') {
            println!("unknown command; try .help");
            continue;
        }

        // Treat as SQL
        let sql = line.to_string();
        let session = app.active_session_mut().unwrap();
        session.query.ensure_tab();
        session.query.active_mut().sql = sql.clone();
        session.main_tab = MainViewTab::CustomQuery;

        let started = std::time::Instant::now();
        match conn
            .execute(&sql, QueryOptions { max_rows: Some(200), ..Default::default() }, CancelToken::new())
            .await
        {
            Ok(result) => {
                let ms = started.elapsed().as_millis() as u64;
                session.console.push(sql.clone(), None, Some(ms));
                session.query.active_mut().results = vec![result.clone()];
                session.query.active_mut().duration_ms = Some(ms);
                session.query.active_mut().error = None;
                if let Some(h) = &history {
                    let _ = h.add_history(Some(&session.id.to_string()), &sql, true, ms, None);
                }
                print_result(&result);
                println!("({} rows, {ms} ms)", result.row_count());
            }
            Err(e) => {
                let ms = started.elapsed().as_millis() as u64;
                session.console.push(sql.clone(), Some(e.to_string()), Some(ms));
                session.query.active_mut().error = Some(e.to_string());
                if let Some(h) = &history {
                    let _ = h.add_history(
                        Some(&session.id.to_string()),
                        &sql,
                        false,
                        ms,
                        Some(&e.to_string()),
                    );
                }
                println!("error: {e}");
            }
        }

        // refresh completions context opportunistically
        let _ = (complete, word_at_cursor, split_statements, CompletionContext::default(), registry());
    }

    let _ = save_settings(&app.settings);
    conn.close().await.ok();
    Ok(())
}

async fn load_table(
    conn: &Arc<dyn ace_driver::Connection>,
    app: &mut AppState,
    db: &str,
    table: &str,
) -> anyhow::Result<()> {
    let meta = conn.table_meta(db, table).await?;
    let fks = conn.list_foreign_keys(db, table).await.unwrap_or_default();
    let triggers = conn.list_triggers(db, table).await.unwrap_or_default();
    let indexes = conn.list_indexes(db, table).await.unwrap_or_default();
    let page_size = app.settings.table.rows_per_page;
    let result = conn
        .fetch_page(db, table, None, None, page_size, 0, CancelToken::new())
        .await?;
    let count = conn
        .count_rows(db, table, None, CancelToken::new())
        .await
        .ok();

    let session = app.active_session_mut().unwrap();
    session.selected_table = Some(table.to_string());
    session.structure.meta = Some(meta.clone());
    session.indexes.indexes = indexes;
    session.relations.foreign_keys = fks;
    session.triggers.triggers = triggers;
    session.table_info = ace_ui::TableInfoViewState {
        engine: meta.engine.clone(),
        charset: meta.charset.clone(),
        collation: meta.collation.clone(),
        comment: meta.comment.clone(),
        auto_increment: meta.auto_increment,
        create_sql: meta.create_sql.clone(),
        row_count: count,
        data_length: None,
    };
    session.content = ace_ui::ContentViewState {
        columns: result.columns.clone(),
        column_meta: meta.columns.clone(),
        rows: result.rows,
        total_rows: count,
        page: 0,
        page_size,
        ..Default::default()
    };
    session.main_tab = MainViewTab::Content;
    println!(
        "table {db}.{table} — {} columns, {:?} rows",
        meta.columns.len(),
        count
    );
    print_content(session);
    Ok(())
}

fn print_content(session: &ace_ui::SessionTab) {
    let c = &session.content;
    if c.columns.is_empty() {
        println!("(no data)");
        return;
    }
    let headers: Vec<_> = c.columns.iter().map(|col| col.name.as_str()).collect();
    println!("{}", headers.join(" | "));
    for (i, row) in c.rows.iter().take(30).enumerate() {
        let cells: Vec<_> = row.iter().map(|v| v.display_preview(24)).collect();
        println!("{i:>3}: {}", cells.join(" | "));
    }
    if c.rows.len() > 30 {
        println!("... {} more rows on this page", c.rows.len() - 30);
    }
}

fn print_result(result: &ace_driver::result::QueryResult) {
    if result.columns.is_empty() {
        println!(
            "OK — affected {} (insert id {:?})",
            result.affected_rows, result.last_insert_id
        );
        return;
    }
    let headers: Vec<_> = result.columns.iter().map(|c| c.name.as_str()).collect();
    println!("{}", headers.join(" | "));
    for row in result.rows.iter().take(50) {
        let cells: Vec<_> = row.iter().map(|v| v.display_preview(32)).collect();
        println!("{}", cells.join(" | "));
    }
}

fn print_help() {
    println!(
        r#"
.db <name>              USE database
.tables                 list tables/views/routines
.use <table>            select table and load content/structure
.structure              show columns
.content                show current page of rows
.relations / .triggers  show FKs / triggers
.tableinfo              show create SQL / engine
.procs / .vars / .users server tools
.explain <sql>          EXPLAIN
.export <file> <sql>    export CSV
.history                recent queries
.tab <name>             switch logical main view
.format                 format current query buffer
.quit                   exit
<sql>                   execute SQL
"#
    );
}
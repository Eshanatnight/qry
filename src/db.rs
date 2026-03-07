use anyhow::{Context, Result};

pub enum DatabaseKind {
    Sqlite,
    DuckDb,
}

pub enum Connection {
    Sqlite(rusqlite::Connection),
    DuckDb(duckdb::Connection),
}

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub struct ColumnInfo {
    pub name: String,
    pub col_type: String,
    pub is_pk: bool,
    pub notnull: bool,
    pub default_value: String,
}

pub struct IndexInfo {
    pub name: String,
    pub unique: bool,
}

pub struct ForeignKeyInfo {
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

impl Connection {
    pub fn open(path: &str, read_write: bool) -> Result<Self> {
        let kind = detect_kind(path);
        match kind {
            DatabaseKind::Sqlite => {
                let conn = rusqlite::Connection::open(path)
                    .with_context(|| format!("Failed to open SQLite database: {path}"))?;
                Ok(Connection::Sqlite(conn))
            }
            DatabaseKind::DuckDb => {
                let mode = if read_write {
                    duckdb::AccessMode::ReadWrite
                } else {
                    duckdb::AccessMode::ReadOnly
                };
                let config = duckdb::Config::default().access_mode(mode)?;
                let conn = duckdb::Connection::open_with_flags(path, config)
                    .with_context(|| format!("failed to open DuckDB database: {path}"))?;
                Ok(Connection::DuckDb(conn))
            }
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Connection::Sqlite(_) => "SQLite",
            Connection::DuckDb(_) => "DuckDB",
        }
    }

    pub fn list_tables(&self) -> Result<Vec<String>> {
        match self {
            Connection::Sqlite(conn) => {
                let mut stmt = conn
                    .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
                let tables = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(tables)
            }
            Connection::DuckDb(conn) => {
                let mut stmt = conn.prepare(
                    "SELECT table_name FROM information_schema.tables WHERE table_schema='main' ORDER BY table_name",
                )?;
                let tables = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(tables)
            }
        }
    }

    pub fn execute_query(&self, sql: &str) -> Result<QueryResult> {
        match self {
            Connection::Sqlite(conn) => execute_sqlite(conn, sql),
            Connection::DuckDb(conn) => execute_duckdb(conn, sql),
        }
    }

    pub fn list_columns(&self, table: &str) -> Result<Vec<String>> {
        let escaped = table.replace('"', "\"\"");
        match self {
            Connection::Sqlite(conn) => {
                let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{escaped}\")"))?;
                let cols = stmt
                    .query_map([], |row| row.get::<_, String>(1))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(cols)
            }
            Connection::DuckDb(_) => {
                let result = self.execute_query(&format!(
                    "SELECT column_name FROM information_schema.columns WHERE table_name='{}' ORDER BY ordinal_position",
                    table.replace('\'', "''")
                ))?;
                Ok(result
                    .rows
                    .into_iter()
                    .filter_map(|r| r.into_iter().next())
                    .collect())
            }
        }
    }

    pub fn table_row_count(&self, table: &str) -> Result<usize> {
        let result = self.execute_query(&format!(
            "SELECT COUNT(*) FROM \"{}\"",
            table.replace('"', "\"\"")
        ))?;
        let count = result
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        Ok(count)
    }

    pub fn list_views(&self) -> Result<Vec<String>> {
        match self {
            Connection::Sqlite(conn) => {
                let mut stmt =
                    conn.prepare("SELECT name FROM sqlite_master WHERE type='view' ORDER BY name")?;
                let views = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(views)
            }
            Connection::DuckDb(conn) => {
                let mut stmt = conn.prepare(
                    "SELECT table_name FROM information_schema.tables WHERE table_schema='main' AND table_type='VIEW' ORDER BY table_name",
                )?;
                let views = stmt
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(views)
            }
        }
    }

    pub fn table_schema(&self, table: &str) -> Result<Vec<ColumnInfo>> {
        let escaped = table.replace('"', "\"\"");
        match self {
            Connection::Sqlite(conn) => {
                let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{escaped}\")"))?;
                let cols = stmt
                    .query_map([], |row| {
                        Ok(ColumnInfo {
                            name: row.get::<_, String>(1)?,
                            col_type: row.get::<_, String>(2)?,
                            notnull: row.get::<_, i32>(3)? != 0,
                            default_value: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                            is_pk: row.get::<_, i32>(5)? != 0,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(cols)
            }
            Connection::DuckDb(_) => {
                let result = self.execute_query(&format!(
                    "SELECT column_name, data_type, is_nullable, COALESCE(column_default, '') FROM information_schema.columns WHERE table_name='{}' ORDER BY ordinal_position",
                    table.replace('\'', "''")
                ))?;
                let cols = result
                    .rows
                    .into_iter()
                    .map(|r| ColumnInfo {
                        name: r.first().cloned().unwrap_or_default(),
                        col_type: r.get(1).cloned().unwrap_or_default(),
                        notnull: r.get(2).map(|v| v == "NO").unwrap_or(false),
                        default_value: r.get(3).cloned().unwrap_or_default(),
                        is_pk: false,
                    })
                    .collect();
                Ok(cols)
            }
        }
    }

    pub fn table_ddl(&self, table: &str) -> Result<String> {
        match self {
            Connection::Sqlite(conn) => {
                let sql: String = conn.query_row(
                    "SELECT sql FROM sqlite_master WHERE type IN ('table', 'view') AND name = ?1",
                    [table],
                    |row| row.get(0),
                )?;
                Ok(sql)
            }
            Connection::DuckDb(_) => {
                let cols = self.table_schema(table)?;
                let mut ddl = format!("CREATE TABLE \"{}\" (\n", table);
                for (i, col) in cols.iter().enumerate() {
                    ddl.push_str(&format!("  \"{}\" {}", col.name, col.col_type));
                    if col.is_pk {
                        ddl.push_str(" PRIMARY KEY");
                    }
                    if col.notnull {
                        ddl.push_str(" NOT NULL");
                    }
                    if !col.default_value.is_empty() {
                        ddl.push_str(&format!(" DEFAULT {}", col.default_value));
                    }
                    if i < cols.len() - 1 {
                        ddl.push(',');
                    }
                    ddl.push('\n');
                }
                ddl.push_str(");");
                Ok(ddl)
            }
        }
    }

    pub fn index_ddl(&self, table: &str) -> Vec<String> {
        match self {
            Connection::Sqlite(conn) => {
                let mut stmt = match conn.prepare(
                    "SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name = ?1 AND sql IS NOT NULL",
                ) {
                    Ok(s) => s,
                    Err(_) => return Vec::new(),
                };
                stmt.query_map([table], |row| row.get::<_, String>(0))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
            }
            Connection::DuckDb(_) => {
                let safe_name = table.replace('\'', "''");
                match self.execute_query(&format!(
                    "SELECT sql FROM duckdb_indexes() WHERE table_name='{safe_name}' AND sql IS NOT NULL"
                )) {
                    Ok(r) => r.rows.into_iter().filter_map(|row| row.into_iter().next()).collect(),
                    Err(_) => Vec::new(),
                }
            }
        }
    }

    pub fn foreign_keys(&self, table: &str) -> Result<Vec<ForeignKeyInfo>> {
        match self {
            Connection::Sqlite(conn) => {
                let escaped = table.replace('"', "\"\"");
                let mut stmt = conn.prepare(&format!("PRAGMA foreign_key_list(\"{escaped}\")"))?;
                let fks = stmt
                    .query_map([], |row| {
                        Ok(ForeignKeyInfo {
                            to_table: row.get::<_, String>(2)?,
                            from_column: row.get::<_, String>(3)?,
                            to_column: row.get::<_, String>(4)?,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(fks)
            }
            Connection::DuckDb(_) => {
                let safe_name = table.replace('\'', "''");
                let result = self.execute_query(&format!(
                    "SELECT kcu.column_name, ccu.table_name, ccu.column_name \
                     FROM information_schema.key_column_usage kcu \
                     JOIN information_schema.constraint_column_usage ccu \
                       ON kcu.constraint_name = ccu.constraint_name \
                     WHERE kcu.table_name = '{safe_name}'"
                ));
                match result {
                    Ok(r) => Ok(r
                        .rows
                        .into_iter()
                        .map(|row| ForeignKeyInfo {
                            from_column: row.first().cloned().unwrap_or_default(),
                            to_table: row.get(1).cloned().unwrap_or_default(),
                            to_column: row.get(2).cloned().unwrap_or_default(),
                        })
                        .collect()),
                    Err(_) => Ok(Vec::new()),
                }
            }
        }
    }

    pub fn list_indexes(&self, table: &str) -> Result<Vec<IndexInfo>> {
        let escaped = table.replace('"', "\"\"");
        match self {
            Connection::Sqlite(conn) => {
                let mut stmt = conn.prepare(&format!("PRAGMA index_list(\"{escaped}\")"))?;
                let indexes = stmt
                    .query_map([], |row| {
                        Ok(IndexInfo {
                            name: row.get::<_, String>(1)?,
                            unique: row.get::<_, i32>(2)? != 0,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(indexes)
            }
            Connection::DuckDb(_) => {
                let result = self.execute_query(&format!(
                    "SELECT index_name, is_unique FROM duckdb_indexes() WHERE table_name='{}'",
                    table.replace('\'', "''")
                ));
                match result {
                    Ok(r) => Ok(r
                        .rows
                        .into_iter()
                        .map(|row| IndexInfo {
                            name: row.first().cloned().unwrap_or_default(),
                            unique: row.get(1).map(|v| v == "true" || v == "t").unwrap_or(false),
                        })
                        .collect()),
                    Err(_) => Ok(Vec::new()),
                }
            }
        }
    }
}

fn detect_kind(path: &str) -> DatabaseKind {
    if let Ok(file) = std::fs::read(path) {
        if file.len() >= 16 && &file[0..16] == b"SQLite format 3\0" {
            return DatabaseKind::Sqlite;
        }
    }
    DatabaseKind::DuckDb
}

fn execute_sqlite(conn: &rusqlite::Connection, sql: &str) -> Result<QueryResult> {
    let mut stmt = conn.prepare(sql).context("SQL prepare error")?;
    let col_count = stmt.column_count();
    let columns: Vec<String> = (0..col_count)
        .map(|i| {
            stmt.column_name(i)
                .map_or("?".to_string(), |v| v.to_string())
        })
        .collect();

    let rows = stmt
        .query_map([], |row| {
            let mut vals = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let val: String = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => "NULL".into(),
                    Ok(rusqlite::types::ValueRef::Integer(v)) => v.to_string(),
                    Ok(rusqlite::types::ValueRef::Real(v)) => v.to_string(),
                    Ok(rusqlite::types::ValueRef::Text(v)) => {
                        String::from_utf8_lossy(v).to_string()
                    }
                    Ok(rusqlite::types::ValueRef::Blob(v)) => format!("<blob {}B>", v.len()),
                    Err(_) => "?".into(),
                };
                vals.push(val);
            }
            Ok(vals)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(QueryResult { columns, rows })
}

fn execute_duckdb(conn: &duckdb::Connection, sql: &str) -> Result<QueryResult> {
    let mut stmt = conn.prepare(sql).context("SQL prepare error")?;
    let mut result_rows = stmt.query([]).context("SQL execute error")?;

    let col_count = result_rows.as_ref().unwrap().column_count();
    let columns: Vec<String> = (0..col_count)
        .map(|i| {
            result_rows
                .as_ref()
                .unwrap()
                .column_name(i)
                .map_or("?", |v| v)
                .to_string()
        })
        .collect();

    let mut rows = Vec::new();
    while let Some(row) = result_rows.next()? {
        let mut vals = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let val: String = match row.get_ref(i) {
                Ok(duckdb::types::ValueRef::Null) => "NULL".into(),
                Ok(duckdb::types::ValueRef::Int(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::BigInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::HugeInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::Float(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::Double(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::Text(v)) => String::from_utf8_lossy(v).to_string(),
                Ok(duckdb::types::ValueRef::Blob(v)) => format!("<blob {}B>", v.len()),
                Ok(duckdb::types::ValueRef::Boolean(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::TinyInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::SmallInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::UInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::UBigInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::UTinyInt(v)) => v.to_string(),
                Ok(duckdb::types::ValueRef::USmallInt(v)) => v.to_string(),
                _ => "?".into(),
            };
            vals.push(val);
        }
        rows.push(vals);
    }

    Ok(QueryResult { columns, rows })
}

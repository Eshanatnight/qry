use crate::db::{ColumnInfo, Connection, ForeignKeyInfo, IndexInfo, QueryResult};
use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const QUERY_ROW_LIMIT: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Tables,
    Data,
    QueryEditor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Table,
    Query,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Filter,
    Export,
    SaveQueryName,
    EditCell,
    DeleteConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Table,
    Column,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub text: String,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone)]
pub struct CompletionState {
    pub candidates: Vec<CompletionItem>,
    pub index: usize,
    pub prefix_start: usize,
}

pub struct SchemaInfo {
    pub table_name: String,
    pub columns: Vec<ColumnInfo>,
    pub create_sql: String,
    pub indexes: Vec<IndexInfo>,
    pub foreign_keys: Vec<ForeignKeyInfo>,
}

pub struct App {
    pub db: Connection,
    pub db_path: String,
    pub tables: Vec<String>,
    pub column_cache: Vec<String>,
    pub selected_table: usize,
    pub table_data: Option<QueryResult>,
    pub data_source: DataSource,
    pub data_scroll_row: usize,
    pub data_scroll_col: usize,
    pub selected_row: usize,
    pub selected_col: usize,
    pub query_input: String,
    pub query_cursor: usize,
    pub query_error: Option<String>,
    pub query_error_token: Option<String>,
    pub focus: Focus,
    pub running: bool,
    pub status_msg: String,
    pub show_help: bool,
    pub help_scroll: usize,
    pub hex_mode: bool,
    pub completion: Option<CompletionState>,

    pub query_history: Vec<String>,
    pub history_index: Option<usize>,
    pub history_saved_input: String,

    pub show_row_detail: bool,
    pub detail_scroll: usize,

    pub show_schema: bool,
    pub schema_info: Option<SchemaInfo>,
    pub schema_scroll: usize,

    pub search_query: String,
    pub input_mode: InputMode,

    pub sort_column: Option<usize>,
    pub sort_ascending: bool,

    pub total_row_count: Option<usize>,
    pub view_start_index: usize,

    pub query_duration: Option<Duration>,

    pub undo_stack: Vec<(String, usize)>,
    pub redo_stack: Vec<(String, usize)>,

    pub filter_query: String,

    pub saved_queries: Vec<(String, String)>,
    pub show_saved_queries: bool,
    pub saved_query_selected: usize,
    pub save_query_name_input: String,

    pub pane_width: u16,

    pub column_width_adj: Vec<i16>,
    pub pinned_columns: usize,

    pub theme_index: usize,

    pub data_dir: Option<PathBuf>,

    pub export_input: String,
    pub export_format: ExportFormat,

    pub edit_buffer: String,
    pub edit_cursor: usize,

    pub table_list_area: Option<ratatui::layout::Rect>,
    pub data_view_area: Option<ratatui::layout::Rect>,
    pub query_editor_area: Option<ratatui::layout::Rect>,
}

impl App {
    pub fn new(db: Connection, db_path: String) -> Result<Self> {
        let tables = db.list_tables()?;
        let views = db.list_views().unwrap_or_default();
        let view_start_index = tables.len();
        let mut all_objects = tables;
        all_objects.extend(views);
        let column_cache = build_column_cache(&db, &all_objects);

        let data_dir = dirs::data_dir().map(|d| d.join("qry"));
        if let Some(ref dir) = data_dir {
            let _ = std::fs::create_dir_all(dir);
        }

        let mut app = Self {
            db,
            db_path,
            tables: all_objects,
            column_cache,
            selected_table: 0,
            table_data: None,
            data_source: DataSource::Table,
            data_scroll_row: 0,
            data_scroll_col: 0,
            selected_row: 0,
            selected_col: 0,
            query_input: String::new(),
            query_cursor: 0,
            query_error: None,
            query_error_token: None,
            focus: Focus::Tables,
            running: true,
            status_msg: String::new(),
            show_help: false,
            help_scroll: 0,
            hex_mode: false,
            completion: None,
            query_history: Vec::new(),
            history_index: None,
            history_saved_input: String::new(),
            show_row_detail: false,
            detail_scroll: 0,
            show_schema: false,
            schema_info: None,
            schema_scroll: 0,
            search_query: String::new(),
            input_mode: InputMode::Normal,
            sort_column: None,
            sort_ascending: true,
            total_row_count: None,
            view_start_index,
            query_duration: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            filter_query: String::new(),
            saved_queries: Vec::new(),
            show_saved_queries: false,
            saved_query_selected: 0,
            save_query_name_input: String::new(),
            pane_width: 24,
            column_width_adj: Vec::new(),
            pinned_columns: 0,
            theme_index: 0,
            data_dir,
            export_input: String::new(),
            export_format: ExportFormat::Csv,
            edit_buffer: String::new(),
            edit_cursor: 0,
            table_list_area: None,
            data_view_area: None,
            query_editor_area: None,
        };
        app.load_history_file();
        app.load_saved_queries_file();
        app.load_selected_table();
        Ok(app)
    }

    // --- Table loading ---

    pub fn load_selected_table(&mut self) {
        if self.tables.is_empty() {
            self.table_data = None;
            self.status_msg = "No tables found".into();
            return;
        }
        let table = &self.tables[self.selected_table];
        let escaped = table.replace('"', "\"\"");
        let sql = format!("SELECT * FROM \"{escaped}\" LIMIT 1000");
        let start = Instant::now();
        match self.db.execute_query(&sql) {
            Ok(result) => {
                self.query_duration = Some(start.elapsed());
                let count = self.db.table_row_count(table).unwrap_or(0);
                let showing = result.rows.len();
                let dur = format_duration(self.query_duration.unwrap());
                self.status_msg = if count > showing {
                    format!("{table}: {showing} of {count} rows (limited) ({dur})")
                } else {
                    format!("{table}: {count} rows ({dur})")
                };
                self.total_row_count = Some(count);
                self.table_data = Some(result);
                self.data_source = DataSource::Table;
                self.data_scroll_row = 0;
                self.data_scroll_col = 0;
                self.selected_row = 0;
                self.selected_col = 0;
                self.sort_column = None;
                self.query_error = None;
                self.query_error_token = None;
                self.column_width_adj.clear();
                self.filter_query.clear();
            }
            Err(e) => {
                self.query_duration = Some(start.elapsed());
                self.table_data = None;
                self.status_msg = format!("Error: {e}");
            }
        }
    }

    // --- Query execution (multi-statement, timing, row limit) ---

    pub fn run_query(&mut self) {
        let sql = self.query_input.trim().to_string();
        if sql.is_empty() {
            return;
        }
        self.push_history(sql.clone());

        let start = Instant::now();
        let statements = split_statements(&sql);

        let mut last_result = None;
        let mut executed = 0;

        for stmt_sql in &statements {
            let trimmed = stmt_sql.trim();
            if trimmed.is_empty() {
                continue;
            }
            let limited = if trimmed.to_uppercase().starts_with("SELECT")
                && !trimmed.to_uppercase().contains("LIMIT")
            {
                format!("{trimmed} LIMIT {QUERY_ROW_LIMIT}")
            } else {
                trimmed.to_string()
            };
            match self.db.execute_query(&limited) {
                Ok(result) => {
                    executed += 1;
                    last_result = Some(result);
                }
                Err(e) => {
                    let err_str = format!("{e}");
                    self.query_error_token = extract_error_token(&err_str);
                    self.query_error = Some(err_str.clone());
                    self.query_duration = Some(start.elapsed());
                    self.status_msg = format!("Query failed: {err_str}");
                    return;
                }
            }
        }

        self.query_duration = Some(start.elapsed());

        if let Some(result) = last_result {
            let row_count = result.rows.len();
            let dur = format_duration(self.query_duration.unwrap());
            if executed > 1 {
                self.status_msg =
                    format!("{executed} statements, {row_count} rows ({dur})");
            } else {
                self.status_msg = format!("Query returned {row_count} rows ({dur})");
            }
            self.total_row_count = Some(row_count);
            self.table_data = Some(result);
            self.data_source = DataSource::Query;
            self.data_scroll_row = 0;
            self.data_scroll_col = 0;
            self.selected_row = 0;
            self.selected_col = 0;
            self.sort_column = None;
            self.query_error = None;
            self.query_error_token = None;
            self.column_width_adj.clear();
            self.focus = Focus::Data;
        }
    }

    pub fn select_table_up(&mut self) {
        if !self.tables.is_empty() && self.selected_table > 0 {
            self.selected_table -= 1;
            self.load_selected_table();
        }
    }

    pub fn select_table_down(&mut self) {
        if !self.tables.is_empty() && self.selected_table < self.tables.len() - 1 {
            self.selected_table += 1;
            self.load_selected_table();
        }
    }

    // --- Data view navigation ---

    pub fn scroll_data_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
            if self.selected_row < self.data_scroll_row {
                self.data_scroll_row = self.selected_row;
            }
        }
    }

    pub fn scroll_data_down(&mut self) {
        if let Some(ref data) = self.table_data {
            if self.selected_row < data.rows.len().saturating_sub(1) {
                self.selected_row += 1;
            }
        }
    }

    pub fn scroll_data_left(&mut self) {
        if self.selected_col > 0 {
            self.selected_col -= 1;
            if self.selected_col < self.data_scroll_col {
                self.data_scroll_col = self.selected_col;
            }
        }
    }

    pub fn scroll_data_right(&mut self) {
        if let Some(ref data) = self.table_data {
            if self.selected_col < data.columns.len().saturating_sub(1) {
                self.selected_col += 1;
            }
        }
    }

    pub fn page_up(&mut self) {
        self.selected_row = self.selected_row.saturating_sub(20);
        if self.selected_row < self.data_scroll_row {
            self.data_scroll_row = self.selected_row;
        }
    }

    pub fn page_down(&mut self) {
        if let Some(ref data) = self.table_data {
            let max = data.rows.len().saturating_sub(1);
            self.selected_row = (self.selected_row + 20).min(max);
        }
    }

    pub fn go_to_first_row(&mut self) {
        self.selected_row = 0;
        self.data_scroll_row = 0;
    }

    pub fn go_to_last_row(&mut self) {
        if let Some(ref data) = self.table_data {
            self.selected_row = data.rows.len().saturating_sub(1);
        }
    }

    pub fn ensure_cursor_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        if self.selected_row < self.data_scroll_row {
            self.data_scroll_row = self.selected_row;
        }
        if self.selected_row >= self.data_scroll_row + visible_height {
            self.data_scroll_row = self.selected_row - visible_height + 1;
        }
    }

    // --- Query editor with undo/redo ---

    fn push_undo(&mut self) {
        self.undo_stack
            .push((self.query_input.clone(), self.query_cursor));
        self.redo_stack.clear();
        if self.undo_stack.len() > 200 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some((input, cursor)) = self.undo_stack.pop() {
            self.redo_stack
                .push((self.query_input.clone(), self.query_cursor));
            self.query_input = input;
            self.query_cursor = cursor;
        }
    }

    pub fn redo(&mut self) {
        if let Some((input, cursor)) = self.redo_stack.pop() {
            self.undo_stack
                .push((self.query_input.clone(), self.query_cursor));
            self.query_input = input;
            self.query_cursor = cursor;
        }
    }

    pub fn query_insert_char(&mut self, c: char) {
        self.push_undo();
        self.query_input.insert(self.query_cursor, c);
        self.query_cursor += c.len_utf8();
    }

    pub fn query_insert_newline(&mut self) {
        self.push_undo();
        self.query_input.insert(self.query_cursor, '\n');
        self.query_cursor += 1;
    }

    pub fn query_backspace(&mut self) {
        if self.query_cursor > 0 {
            self.push_undo();
            let prev = self.query_input[..self.query_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.query_input.drain(prev..self.query_cursor);
            self.query_cursor = prev;
        }
    }

    pub fn query_delete(&mut self) {
        if self.query_cursor < self.query_input.len() {
            self.push_undo();
            let next = self.query_input[self.query_cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.query_cursor + i)
                .unwrap_or(self.query_input.len());
            self.query_input.drain(self.query_cursor..next);
        }
    }

    pub fn query_move_left(&mut self) {
        if self.query_cursor > 0 {
            self.query_cursor = self.query_input[..self.query_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn query_move_right(&mut self) {
        if self.query_cursor < self.query_input.len() {
            self.query_cursor = self.query_input[self.query_cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.query_cursor + i)
                .unwrap_or(self.query_input.len());
        }
    }

    pub fn query_move_home(&mut self) {
        self.query_cursor = 0;
    }

    pub fn query_move_end(&mut self) {
        self.query_cursor = self.query_input.len();
    }

    pub fn query_clear(&mut self) {
        self.push_undo();
        self.query_input.clear();
        self.query_cursor = 0;
    }

    // --- Focus cycling ---

    pub fn cycle_focus_forward(&mut self) {
        self.focus = match self.focus {
            Focus::Tables => Focus::Data,
            Focus::Data => Focus::QueryEditor,
            Focus::QueryEditor => Focus::Tables,
        };
    }

    pub fn cycle_focus_backward(&mut self) {
        self.focus = match self.focus {
            Focus::Tables => Focus::QueryEditor,
            Focus::Data => Focus::Tables,
            Focus::QueryEditor => Focus::Data,
        };
    }

    // --- Refresh ---

    pub fn refresh_tables(&mut self) {
        if let Ok(tables) = self.db.list_tables() {
            let views = self.db.list_views().unwrap_or_default();
            self.view_start_index = tables.len();
            let mut all = tables;
            all.extend(views);
            self.tables = all;
            self.column_cache = build_column_cache(&self.db, &self.tables);
            if self.selected_table >= self.tables.len() {
                self.selected_table = self.tables.len().saturating_sub(1);
            }
            self.load_selected_table();
            self.status_msg = "Refreshed".into();
        }
    }

    // --- Hex mode ---

    pub fn toggle_hex_mode(&mut self) {
        self.hex_mode = !self.hex_mode;
        self.status_msg = if self.hex_mode {
            "Hex display ON".into()
        } else {
            "Hex display OFF".into()
        };
    }

    pub fn format_cell_value<'a>(&self, val: &'a str) -> std::borrow::Cow<'a, str> {
        if !self.hex_mode {
            return std::borrow::Cow::Borrowed(val);
        }
        if val == "NULL" || val.is_empty() {
            return std::borrow::Cow::Borrowed(val);
        }
        if let Ok(v) = val.parse::<i64>() {
            if v >= 0 {
                return std::borrow::Cow::Owned(format!("0x{:X}", v));
            } else {
                return std::borrow::Cow::Owned(format!("-0x{:X}", v.unsigned_abs()));
            }
        }
        if let Ok(v) = val.parse::<f64>() {
            let i = v as i64;
            if (v - i as f64).abs() < f64::EPSILON {
                if i >= 0 {
                    return std::borrow::Cow::Owned(format!("0x{:X}", i));
                } else {
                    return std::borrow::Cow::Owned(format!("-0x{:X}", i.unsigned_abs()));
                }
            }
        }
        std::borrow::Cow::Borrowed(val)
    }

    // --- Fuzzy completion ---

    pub fn dismiss_completion(&mut self) {
        self.completion = None;
    }

    pub fn trigger_completion(&mut self) {
        let (prefix, prefix_start) =
            extract_word_before_cursor(&self.query_input, self.query_cursor);
        if prefix.is_empty() {
            self.completion = None;
            return;
        }

        let mut candidates: Vec<(CompletionItem, i32)> = Vec::new();

        for &kw in SQL_KEYWORDS {
            if let Some(score) = fuzzy_match(kw, &prefix) {
                candidates.push((
                    CompletionItem {
                        text: kw.to_string(),
                        kind: CompletionKind::Keyword,
                    },
                    score,
                ));
            }
        }
        for table in &self.tables {
            if let Some(score) = fuzzy_match(table, &prefix) {
                candidates.push((
                    CompletionItem {
                        text: table.clone(),
                        kind: CompletionKind::Table,
                    },
                    score,
                ));
            }
        }
        for col in &self.column_cache {
            if let Some(score) = fuzzy_match(col, &prefix) {
                candidates.push((
                    CompletionItem {
                        text: col.clone(),
                        kind: CompletionKind::Column,
                    },
                    score,
                ));
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.text.len().cmp(&b.0.text.len())));
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|(item, _)| seen.insert(item.text.to_ascii_lowercase()));
        let items: Vec<CompletionItem> = candidates.into_iter().map(|(item, _)| item).collect();

        if items.is_empty() {
            self.completion = None;
            return;
        }

        let state = CompletionState {
            candidates: items,
            index: 0,
            prefix_start,
        };
        self.completion = Some(state);
        self.apply_completion();
    }

    pub fn cycle_completion(&mut self) {
        if let Some(ref mut state) = self.completion {
            state.index = (state.index + 1) % state.candidates.len();
        }
        self.apply_completion();
    }

    pub fn cycle_completion_back(&mut self) {
        if let Some(ref mut state) = self.completion {
            if state.index == 0 {
                state.index = state.candidates.len() - 1;
            } else {
                state.index -= 1;
            }
        }
        self.apply_completion();
    }

    fn apply_completion(&mut self) {
        let Some(ref state) = self.completion else {
            return;
        };
        let replacement = &state.candidates[state.index].text;
        let start = state.prefix_start;
        let old_end = self.query_cursor;
        self.query_input.replace_range(start..old_end, replacement);
        self.query_cursor = start + replacement.len();
    }

    // --- Query history (persistent) ---

    fn push_history(&mut self, query: String) {
        if self.query_history.last().map(|h| h.as_str()) != Some(query.as_str()) {
            self.query_history.push(query);
        }
        self.history_index = None;
        self.history_saved_input.clear();
    }

    pub fn history_up(&mut self) {
        if self.query_history.is_empty() {
            return;
        }
        match self.history_index {
            None => {
                self.history_saved_input = self.query_input.clone();
                self.history_index = Some(self.query_history.len() - 1);
            }
            Some(0) => return,
            Some(ref mut idx) => *idx -= 1,
        }
        if let Some(idx) = self.history_index {
            self.query_input = self.query_history[idx].clone();
            self.query_cursor = self.query_input.len();
        }
    }

    pub fn history_down(&mut self) {
        match self.history_index {
            None => return,
            Some(idx) => {
                if idx >= self.query_history.len() - 1 {
                    self.history_index = None;
                    self.query_input = self.history_saved_input.clone();
                    self.query_cursor = self.query_input.len();
                } else {
                    self.history_index = Some(idx + 1);
                    self.query_input = self.query_history[idx + 1].clone();
                    self.query_cursor = self.query_input.len();
                }
            }
        }
    }

    pub fn load_history_file(&mut self) {
        if let Some(ref dir) = self.data_dir {
            let path = dir.join("history");
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.query_history = content
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
            }
        }
    }

    pub fn save_history_file(&self) {
        if let Some(ref dir) = self.data_dir {
            let path = dir.join("history");
            let last_500: Vec<&str> = self
                .query_history
                .iter()
                .rev()
                .take(500)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|s| s.as_str())
                .collect();
            let content = last_500.join("\0");
            let _ = std::fs::write(&path, content);
        }
    }

    // --- Row detail view ---

    pub fn toggle_row_detail(&mut self) {
        if self.table_data.is_some() {
            self.show_row_detail = !self.show_row_detail;
            self.detail_scroll = 0;
        }
    }

    // --- Schema viewer (with FK) ---

    pub fn load_schema(&mut self) {
        if self.tables.is_empty() {
            return;
        }
        let table = &self.tables[self.selected_table].clone();
        let columns = self.db.table_schema(table).unwrap_or_default();
        let mut create_sql = self.db.table_ddl(table).unwrap_or_else(|_| "N/A".into());
        let index_ddls = self.db.index_ddl(table);
        for ddl in &index_ddls {
            create_sql.push_str("\n\n");
            create_sql.push_str(ddl);
            if !ddl.ends_with(';') {
                create_sql.push(';');
            }
        }
        let indexes = self.db.list_indexes(table).unwrap_or_default();
        let foreign_keys = self.db.foreign_keys(table).unwrap_or_default();
        self.schema_info = Some(SchemaInfo {
            table_name: table.clone(),
            columns,
            create_sql,
            indexes,
            foreign_keys,
        });
        self.show_schema = true;
        self.schema_scroll = 0;
    }

    // --- Sorting ---

    pub fn toggle_sort(&mut self) {
        let col = self.selected_col;
        if self.sort_column == Some(col) {
            if self.sort_ascending {
                self.sort_ascending = false;
            } else {
                self.sort_column = None;
                self.sort_ascending = true;
                self.status_msg = "Sort cleared (refresh to restore original order)".into();
                return;
            }
        } else {
            self.sort_column = Some(col);
            self.sort_ascending = true;
        }
        self.sort_data();
        if let Some(col_idx) = self.sort_column {
            if let Some(ref data) = self.table_data {
                if col_idx < data.columns.len() {
                    let col_name = &data.columns[col_idx];
                    let dir = if self.sort_ascending { "ASC" } else { "DESC" };
                    self.status_msg = format!("Sorted by {col_name} {dir}");
                }
            }
        }
    }

    fn sort_data(&mut self) {
        let Some(col) = self.sort_column else { return };
        let asc = self.sort_ascending;
        if let Some(ref mut data) = self.table_data {
            if col < data.columns.len() {
                data.rows.sort_by(|a, b| {
                    let va = a.get(col).map(|s| s.as_str()).unwrap_or("");
                    let vb = b.get(col).map(|s| s.as_str()).unwrap_or("");
                    let cmp = match (va.parse::<f64>(), vb.parse::<f64>()) {
                        (Ok(na), Ok(nb)) => {
                            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => va.to_lowercase().cmp(&vb.to_lowercase()),
                    };
                    if asc {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
        }
    }

    // --- Search ---

    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_query.clear();
    }

    pub fn end_search(&mut self) {
        self.input_mode = InputMode::Normal;
        if self.search_query.is_empty() {
            self.status_msg = "Search cleared".into();
        } else {
            self.status_msg = format!("Search: \"{}\" (n/N to navigate)", self.search_query);
        }
    }

    pub fn cancel_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.status_msg = String::new();
    }

    pub fn search_insert_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
    }

    pub fn search_next(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let Some(ref data) = self.table_data else {
            return;
        };
        let lower_query = self.search_query.to_lowercase();
        let start_row = self.selected_row + 1;
        for ri in 0..data.rows.len() {
            let row_idx = (start_row + ri) % data.rows.len();
            for val in &data.rows[row_idx] {
                if val.to_lowercase().contains(&lower_query) {
                    self.selected_row = row_idx;
                    self.status_msg =
                        format!("Match at row {} for \"{}\"", row_idx + 1, self.search_query);
                    return;
                }
            }
        }
        self.status_msg = format!("No match for \"{}\"", self.search_query);
    }

    pub fn search_prev(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let Some(ref data) = self.table_data else {
            return;
        };
        let lower_query = self.search_query.to_lowercase();
        let total = data.rows.len();
        for ri in 1..=total {
            let row_idx = (self.selected_row + total - ri) % total;
            for val in &data.rows[row_idx] {
                if val.to_lowercase().contains(&lower_query) {
                    self.selected_row = row_idx;
                    self.status_msg =
                        format!("Match at row {} for \"{}\"", row_idx + 1, self.search_query);
                    return;
                }
            }
        }
        self.status_msg = format!("No match for \"{}\"", self.search_query);
    }

    // --- Filter bar ---

    pub fn start_filter(&mut self) {
        self.input_mode = InputMode::Filter;
    }

    pub fn end_filter(&mut self) {
        self.input_mode = InputMode::Normal;
        if self.filter_query.is_empty() {
            self.status_msg = "Filter cleared".into();
            self.load_selected_table();
        } else {
            self.apply_filter();
        }
    }

    pub fn cancel_filter(&mut self) {
        self.input_mode = InputMode::Normal;
        self.filter_query.clear();
        self.load_selected_table();
        self.status_msg = "Filter cleared".into();
    }

    pub fn filter_insert_char(&mut self, c: char) {
        self.filter_query.push(c);
    }

    pub fn filter_backspace(&mut self) {
        self.filter_query.pop();
    }

    fn apply_filter(&mut self) {
        if self.data_source != DataSource::Table || self.tables.is_empty() {
            self.status_msg = "Filter only works on table data".into();
            return;
        }
        let table = &self.tables[self.selected_table];
        let escaped = table.replace('"', "\"\"");
        let sql = format!(
            "SELECT * FROM \"{escaped}\" WHERE {} LIMIT 1000",
            self.filter_query
        );
        let start = Instant::now();
        match self.db.execute_query(&sql) {
            Ok(result) => {
                self.query_duration = Some(start.elapsed());
                let dur = format_duration(self.query_duration.unwrap());
                self.status_msg = format!(
                    "{table}: {} rows matching filter ({dur})",
                    result.rows.len()
                );
                self.total_row_count = Some(result.rows.len());
                self.table_data = Some(result);
                self.data_scroll_row = 0;
                self.data_scroll_col = 0;
                self.selected_row = 0;
                self.selected_col = 0;
                self.sort_column = None;
            }
            Err(e) => {
                self.status_msg = format!("Filter error: {e}");
            }
        }
    }

    // --- Cell value / yank ---

    pub fn yank_cell(&mut self) {
        if let Some(ref data) = self.table_data {
            if let Some(row) = data.rows.get(self.selected_row) {
                if let Some(val) = row.get(self.selected_col) {
                    let col_name = data
                        .columns
                        .get(self.selected_col)
                        .map(|s| s.as_str())
                        .unwrap_or("?");
                    self.status_msg = format!("Copied {col_name}: {val}");
                    #[cfg(not(test))]
                    {
                        let _ = copy_to_clipboard(val);
                    }
                    return;
                }
            }
        }
        self.status_msg = "No cell to copy".into();
    }

    pub fn yank_row(&mut self) {
        if let Some(ref data) = self.table_data {
            if let Some(row) = data.rows.get(self.selected_row) {
                let line = row.join("\t");
                self.status_msg = format!("Copied row {} ({} cols)", self.selected_row + 1, row.len());
                #[cfg(not(test))]
                {
                    let _ = copy_to_clipboard(&line);
                }
                return;
            }
        }
        self.status_msg = "No row to copy".into();
    }

    pub fn yank_column(&mut self) {
        if let Some(ref data) = self.table_data {
            let col = self.selected_col;
            if col < data.columns.len() {
                let vals: Vec<&str> = data
                    .rows
                    .iter()
                    .filter_map(|row| row.get(col).map(|s| s.as_str()))
                    .collect();
                let text = vals.join("\n");
                let col_name = &data.columns[col];
                self.status_msg =
                    format!("Copied column {col_name} ({} values)", vals.len());
                #[cfg(not(test))]
                {
                    let _ = copy_to_clipboard(&text);
                }
                return;
            }
        }
        self.status_msg = "No column to copy".into();
    }

    // --- Export (custom path) ---

    pub fn start_export_csv(&mut self) {
        self.export_format = ExportFormat::Csv;
        let filename = match self.data_source {
            DataSource::Table if !self.tables.is_empty() => {
                format!("{}_export.csv", self.tables[self.selected_table])
            }
            _ => "query_export.csv".to_string(),
        };
        self.export_input = filename;
        self.input_mode = InputMode::Export;
    }

    pub fn start_export_json(&mut self) {
        self.export_format = ExportFormat::Json;
        let filename = match self.data_source {
            DataSource::Table if !self.tables.is_empty() => {
                format!("{}_export.json", self.tables[self.selected_table])
            }
            _ => "query_export.json".to_string(),
        };
        self.export_input = filename;
        self.input_mode = InputMode::Export;
    }

    pub fn confirm_export(&mut self) {
        self.input_mode = InputMode::Normal;
        let filename = self.export_input.clone();
        match self.export_format {
            ExportFormat::Csv => self.do_export_csv(&filename),
            ExportFormat::Json => self.do_export_json(&filename),
        }
    }

    pub fn cancel_export(&mut self) {
        self.input_mode = InputMode::Normal;
        self.export_input.clear();
    }

    pub fn export_insert_char(&mut self, c: char) {
        self.export_input.push(c);
    }

    pub fn export_backspace(&mut self) {
        self.export_input.pop();
    }

    fn do_export_csv(&mut self, filename: &str) {
        let Some(ref data) = self.table_data else {
            self.status_msg = "No data to export".into();
            return;
        };
        let mut out = String::new();
        out.push_str(
            &data
                .columns
                .iter()
                .map(|c| escape_csv(c))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
        for row in &data.rows {
            out.push_str(
                &row.iter()
                    .map(|v| escape_csv(v))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            out.push('\n');
        }
        match std::fs::write(filename, &out) {
            Ok(_) => {
                self.status_msg = format!("Exported {} rows to {filename}", data.rows.len());
            }
            Err(e) => {
                self.status_msg = format!("Export failed: {e}");
            }
        }
    }

    fn do_export_json(&mut self, filename: &str) {
        let Some(ref data) = self.table_data else {
            self.status_msg = "No data to export".into();
            return;
        };
        let mut out = String::from("[\n");
        for (ri, row) in data.rows.iter().enumerate() {
            out.push_str("  {");
            for (ci, val) in row.iter().enumerate() {
                if ci > 0 {
                    out.push_str(", ");
                }
                let col = &data.columns[ci];
                out.push_str(&format!("\"{}\": ", escape_json(col)));
                if val == "NULL" {
                    out.push_str("null");
                } else if val.parse::<i64>().is_ok() {
                    out.push_str(val);
                } else if val.parse::<f64>().is_ok() {
                    out.push_str(val);
                } else if val == "true" || val == "false" {
                    out.push_str(val);
                } else {
                    out.push_str(&format!("\"{}\"", escape_json(val)));
                }
            }
            out.push('}');
            if ri < data.rows.len() - 1 {
                out.push(',');
            }
            out.push('\n');
        }
        out.push(']');
        match std::fs::write(filename, &out) {
            Ok(_) => {
                self.status_msg = format!("Exported {} rows to {filename}", data.rows.len());
            }
            Err(e) => {
                self.status_msg = format!("Export failed: {e}");
            }
        }
    }

    // --- Edit mode (inline cell editing) ---

    pub fn start_edit(&mut self) {
        if self.data_source != DataSource::Table {
            self.status_msg = "Edit only works on table data (not query results)".into();
            return;
        }
        if let Some(ref data) = self.table_data {
            if let Some(row) = data.rows.get(self.selected_row) {
                if let Some(val) = row.get(self.selected_col) {
                    self.edit_buffer = if val == "NULL" {
                        String::new()
                    } else {
                        val.clone()
                    };
                    self.edit_cursor = self.edit_buffer.len();
                    self.input_mode = InputMode::EditCell;
                    return;
                }
            }
        }
        self.status_msg = "No cell to edit".into();
    }

    pub fn edit_insert_char(&mut self, c: char) {
        self.edit_buffer.insert(self.edit_cursor, c);
        self.edit_cursor += c.len_utf8();
    }

    pub fn edit_backspace(&mut self) {
        if self.edit_cursor > 0 {
            let prev = self.edit_buffer[..self.edit_cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.edit_buffer.drain(prev..self.edit_cursor);
            self.edit_cursor = prev;
        }
    }

    pub fn confirm_edit(&mut self) {
        self.input_mode = InputMode::Normal;
        if self.tables.is_empty() {
            return;
        }
        let table = self.tables[self.selected_table].clone();
        let Some(ref data) = self.table_data else { return };
        let Some(row) = data.rows.get(self.selected_row) else { return };
        let col_name = data.columns.get(self.selected_col).cloned().unwrap_or_default();

        let escaped_table = table.replace('"', "\"\"");
        let escaped_col = col_name.replace('"', "\"\"");
        let new_val = self.edit_buffer.replace('\'', "''");
        let where_clause = build_where_clause(&data.columns, row);

        let sql = format!(
            "UPDATE \"{escaped_table}\" SET \"{escaped_col}\" = '{new_val}' WHERE {where_clause}"
        );

        match self.db.execute_query(&sql) {
            Ok(_) => {
                self.status_msg = format!("Updated {col_name}");
                self.load_selected_table();
            }
            Err(e) => {
                self.status_msg = format!("Update failed: {e}");
            }
        }
    }

    pub fn cancel_edit(&mut self) {
        self.input_mode = InputMode::Normal;
        self.edit_buffer.clear();
    }

    // --- Delete row ---

    pub fn start_delete_confirm(&mut self) {
        if self.data_source != DataSource::Table {
            self.status_msg = "Delete only works on table data".into();
            return;
        }
        if self.table_data.as_ref().map(|d| d.rows.is_empty()).unwrap_or(true) {
            self.status_msg = "No row to delete".into();
            return;
        }
        self.input_mode = InputMode::DeleteConfirm;
        self.status_msg = "Delete this row? (y/n)".into();
    }

    pub fn confirm_delete(&mut self) {
        self.input_mode = InputMode::Normal;
        if self.tables.is_empty() {
            return;
        }
        let table = self.tables[self.selected_table].clone();
        let Some(ref data) = self.table_data else { return };
        let Some(row) = data.rows.get(self.selected_row) else { return };

        let escaped_table = table.replace('"', "\"\"");
        let where_clause = build_where_clause(&data.columns, row);

        let sql = format!("DELETE FROM \"{escaped_table}\" WHERE {where_clause} LIMIT 1");
        match self.db.execute_query(&sql) {
            Ok(_) => {
                self.status_msg = "Row deleted".into();
                self.load_selected_table();
            }
            Err(_) => {
                let sql_no_limit =
                    format!("DELETE FROM \"{escaped_table}\" WHERE {where_clause}");
                match self.db.execute_query(&sql_no_limit) {
                    Ok(_) => {
                        self.status_msg = "Row deleted".into();
                        self.load_selected_table();
                    }
                    Err(e) => {
                        self.status_msg = format!("Delete failed: {e}");
                    }
                }
            }
        }
    }

    pub fn cancel_delete(&mut self) {
        self.input_mode = InputMode::Normal;
        self.status_msg = "Delete cancelled".into();
    }

    // --- Saved queries ---

    pub fn toggle_saved_queries(&mut self) {
        self.show_saved_queries = !self.show_saved_queries;
        self.saved_query_selected = 0;
    }

    pub fn start_save_query(&mut self) {
        if self.query_input.trim().is_empty() {
            self.status_msg = "No query to save".into();
            return;
        }
        self.save_query_name_input.clear();
        self.input_mode = InputMode::SaveQueryName;
    }

    pub fn save_query_insert_char(&mut self, c: char) {
        self.save_query_name_input.push(c);
    }

    pub fn save_query_backspace(&mut self) {
        self.save_query_name_input.pop();
    }

    pub fn confirm_save_query(&mut self) {
        self.input_mode = InputMode::Normal;
        let name = self.save_query_name_input.trim().to_string();
        if name.is_empty() {
            self.status_msg = "Save cancelled (empty name)".into();
            return;
        }
        let sql = self.query_input.clone();
        self.saved_queries.retain(|(n, _)| n != &name);
        self.saved_queries.push((name.clone(), sql));
        self.save_saved_queries_file();
        self.status_msg = format!("Saved query as \"{name}\"");
    }

    pub fn cancel_save_query(&mut self) {
        self.input_mode = InputMode::Normal;
        self.save_query_name_input.clear();
    }

    pub fn load_saved_query(&mut self) {
        if let Some((_, sql)) = self.saved_queries.get(self.saved_query_selected).cloned() {
            self.query_input = sql;
            self.query_cursor = self.query_input.len();
            self.show_saved_queries = false;
            self.focus = Focus::QueryEditor;
            self.status_msg = "Loaded saved query".into();
        }
    }

    pub fn delete_saved_query(&mut self) {
        if self.saved_query_selected < self.saved_queries.len() {
            let name = self.saved_queries[self.saved_query_selected].0.clone();
            self.saved_queries.remove(self.saved_query_selected);
            self.save_saved_queries_file();
            if self.saved_query_selected > 0 {
                self.saved_query_selected -= 1;
            }
            self.status_msg = format!("Deleted saved query \"{name}\"");
        }
    }

    fn load_saved_queries_file(&mut self) {
        if let Some(ref dir) = self.data_dir {
            let path = dir.join("saved_queries");
            if let Ok(content) = std::fs::read_to_string(&path) {
                self.saved_queries = content
                    .split('\x1e')
                    .filter(|s| !s.is_empty())
                    .filter_map(|entry| {
                        let mut parts = entry.splitn(2, '\x1f');
                        let name = parts.next()?.to_string();
                        let sql = parts.next()?.to_string();
                        Some((name, sql))
                    })
                    .collect();
            }
        }
    }

    fn save_saved_queries_file(&self) {
        if let Some(ref dir) = self.data_dir {
            let path = dir.join("saved_queries");
            let entries: Vec<String> = self
                .saved_queries
                .iter()
                .map(|(name, sql)| format!("{}\x1f{}", name, sql))
                .collect();
            let content = entries.join("\x1e");
            let _ = std::fs::write(&path, content);
        }
    }

    // --- Column resize ---

    pub fn widen_column(&mut self) {
        let col = self.selected_col;
        if self.column_width_adj.len() <= col {
            self.column_width_adj.resize(col + 1, 0);
        }
        self.column_width_adj[col] += 4;
    }

    pub fn narrow_column(&mut self) {
        let col = self.selected_col;
        if self.column_width_adj.len() <= col {
            self.column_width_adj.resize(col + 1, 0);
        }
        self.column_width_adj[col] = (self.column_width_adj[col] - 4).max(-20);
    }

    pub fn column_adj(&self, col: usize) -> i16 {
        self.column_width_adj.get(col).copied().unwrap_or(0)
    }

    // --- Pin columns ---

    pub fn toggle_pin(&mut self) {
        if self.pinned_columns > 0 {
            self.pinned_columns = 0;
            self.status_msg = "Columns unpinned".into();
        } else {
            self.pinned_columns = (self.selected_col + 1).min(3);
            self.status_msg = format!("Pinned first {} column(s)", self.pinned_columns);
        }
    }

    // --- Pane resize ---

    pub fn grow_pane(&mut self) {
        self.pane_width = (self.pane_width + 2).min(60);
    }

    pub fn shrink_pane(&mut self) {
        self.pane_width = self.pane_width.saturating_sub(2).max(12);
    }

    // --- Theme ---

    pub fn cycle_theme(&mut self) {
        self.theme_index = (self.theme_index + 1) % 3;
    }

    // --- Mouse helpers ---

    pub fn click_at(&mut self, col: u16, row: u16) {
        if let Some(area) = self.table_list_area {
            if col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height {
                let inner_row = (row - area.y).saturating_sub(1) as usize;
                if inner_row < self.tables.len() {
                    self.selected_table = inner_row;
                    self.load_selected_table();
                    self.focus = Focus::Tables;
                }
                return;
            }
        }
        if let Some(area) = self.data_view_area {
            if col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height {
                let inner_row = (row - area.y).saturating_sub(2) as usize;
                let abs_row = self.data_scroll_row + inner_row;
                if let Some(ref data) = self.table_data {
                    if abs_row < data.rows.len() {
                        self.selected_row = abs_row;
                    }
                }
                self.focus = Focus::Data;
                return;
            }
        }
        if let Some(area) = self.query_editor_area {
            if col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height {
                self.focus = Focus::QueryEditor;
                return;
            }
        }
    }

    pub fn scroll_at(&mut self, col: u16, row: u16, down: bool) {
        if let Some(area) = self.table_list_area {
            if col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height {
                if down {
                    self.select_table_down();
                } else {
                    self.select_table_up();
                }
                return;
            }
        }
        if let Some(area) = self.data_view_area {
            if col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height {
                if down {
                    self.scroll_data_down();
                    self.scroll_data_down();
                    self.scroll_data_down();
                } else {
                    self.scroll_data_up();
                    self.scroll_data_up();
                    self.scroll_data_up();
                }
                return;
            }
        }
    }
}

// --- Helper functions ---

fn escape_csv(val: &str) -> String {
    if val.contains(',') || val.contains('"') || val.contains('\n') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val.to_string()
    }
}

fn escape_json(val: &str) -> String {
    val.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    let commands: &[(&str, &[&str])] = &[
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("wl-copy", &[]),
        ("pbcopy", &[]),
        ("clip.exe", &[]),
    ];
    for &(cmd, args) in commands {
        if let Ok(mut child) = std::process::Command::new(cmd)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return Ok(());
        }
    }
    Ok(())
}

fn build_column_cache(db: &Connection, tables: &[String]) -> Vec<String> {
    let mut cols = Vec::new();
    for table in tables {
        if let Ok(table_cols) = db.list_columns(table) {
            for c in table_cols {
                if !cols
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(&c))
                {
                    cols.push(c);
                }
            }
        }
    }
    cols.sort_by_key(|a| a.to_ascii_lowercase());
    cols
}

fn extract_word_before_cursor(input: &str, cursor: usize) -> (String, usize) {
    let before = &input[..cursor];
    let start = before
        .rfind(|c: char| c.is_whitespace() || "(),;".contains(c))
        .map(|i| i + before[i..].chars().next().unwrap().len_utf8())
        .unwrap_or(0);
    (before[start..].to_string(), start)
}

fn fuzzy_match(text: &str, query: &str) -> Option<i32> {
    let text_lower = text.to_ascii_lowercase();
    let query_lower = query.to_ascii_lowercase();
    if text_lower == query_lower {
        return None;
    }
    if text_lower.starts_with(&query_lower) {
        return Some(1000 - text.len() as i32);
    }
    if text_lower.contains(&query_lower) {
        return Some(500 - text.len() as i32);
    }
    let mut qi = query_lower.chars().peekable();
    let mut score = 0i32;
    for ch in text_lower.chars() {
        if let Some(&qch) = qi.peek() {
            if ch == qch {
                score += 10;
                qi.next();
            }
        }
    }
    if qi.peek().is_none() {
        Some(score - text.len() as i32)
    } else {
        None
    }
}

fn split_statements(sql: &str) -> Vec<String> {
    let mut stmts = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    for ch in sql.chars() {
        if ch == '\'' {
            in_string = !in_string;
            current.push(ch);
        } else if ch == ';' && !in_string {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                stmts.push(trimmed);
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        stmts.push(trimmed);
    }
    stmts
}

fn extract_error_token(error: &str) -> Option<String> {
    if let Some(start) = error.find("near \"") {
        let rest = &error[start + 6..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    if let Some(start) = error.find("near \"") {
        let rest = &error[start + 6..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn build_where_clause(columns: &[String], row: &[String]) -> String {
    columns
        .iter()
        .zip(row.iter())
        .map(|(col, val)| {
            let escaped_col = col.replace('"', "\"\"");
            if val == "NULL" {
                format!("\"{escaped_col}\" IS NULL")
            } else {
                let escaped_val = val.replace('\'', "''");
                format!("\"{escaped_col}\" = '{escaped_val}'")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub fn format_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros < 1000 {
        format!("{micros}µs")
    } else if micros < 1_000_000 {
        format!("{:.1}ms", micros as f64 / 1000.0)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

const SQL_KEYWORDS: &[&str] = &[
    "ALTER", "AND", "AS", "ASC", "AVG", "BETWEEN", "BY", "CASE", "CAST", "COALESCE", "COUNT",
    "CREATE", "CROSS", "DELETE", "DESC", "DISTINCT", "DROP", "ELSE", "END", "EXISTS", "EXPLAIN",
    "FALSE", "FROM", "FULL", "GROUP", "HAVING", "IF", "IN", "INDEX", "INNER", "INSERT", "INTO",
    "IS", "JOIN", "LEFT", "LIKE", "LIMIT", "MAX", "MIN", "NOT", "NULL", "OFFSET", "ON", "OR",
    "ORDER", "OUTER", "PRIMARY", "RIGHT", "SELECT", "SET", "SUM", "TABLE", "THEN", "TRUE",
    "UNION", "UPDATE", "USING", "VALUES", "WHEN", "WHERE", "WITH",
];

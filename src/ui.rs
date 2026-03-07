use crate::app::{App, CompletionKind, DataSource, Focus, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};

pub struct Theme {
    pub name: &'static str,
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub overlay0: Color,
    pub text: Color,
    pub subtext0: Color,
    pub accent: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub lavender: Color,
    pub peach: Color,
    pub teal: Color,
}

const THEMES: [Theme; 3] = [
    Theme {
        name: "Catppuccin Mocha",
        base: Color::Rgb(30, 30, 46),
        mantle: Color::Rgb(24, 24, 37),
        crust: Color::Rgb(17, 17, 27),
        surface0: Color::Rgb(49, 50, 68),
        surface1: Color::Rgb(69, 71, 90),
        overlay0: Color::Rgb(108, 112, 134),
        text: Color::Rgb(205, 214, 244),
        subtext0: Color::Rgb(166, 173, 200),
        accent: Color::Rgb(203, 166, 247),
        red: Color::Rgb(243, 139, 168),
        green: Color::Rgb(166, 227, 161),
        yellow: Color::Rgb(249, 226, 175),
        blue: Color::Rgb(137, 180, 250),
        lavender: Color::Rgb(180, 190, 254),
        peach: Color::Rgb(250, 179, 135),
        teal: Color::Rgb(148, 226, 213),
    },
    Theme {
        name: "Tokyo Night",
        base: Color::Rgb(26, 27, 38),
        mantle: Color::Rgb(22, 22, 30),
        crust: Color::Rgb(15, 15, 20),
        surface0: Color::Rgb(41, 46, 66),
        surface1: Color::Rgb(59, 66, 97),
        overlay0: Color::Rgb(96, 104, 137),
        text: Color::Rgb(192, 202, 245),
        subtext0: Color::Rgb(150, 160, 200),
        accent: Color::Rgb(122, 162, 247),
        red: Color::Rgb(247, 118, 142),
        green: Color::Rgb(158, 206, 106),
        yellow: Color::Rgb(224, 175, 104),
        blue: Color::Rgb(122, 162, 247),
        lavender: Color::Rgb(187, 154, 247),
        peach: Color::Rgb(255, 158, 100),
        teal: Color::Rgb(115, 218, 202),
    },
    Theme {
        name: "Catppuccin Latte",
        base: Color::Rgb(239, 241, 245),
        mantle: Color::Rgb(230, 233, 239),
        crust: Color::Rgb(220, 224, 232),
        surface0: Color::Rgb(204, 208, 218),
        surface1: Color::Rgb(188, 192, 204),
        overlay0: Color::Rgb(140, 143, 161),
        text: Color::Rgb(76, 79, 105),
        subtext0: Color::Rgb(108, 111, 133),
        accent: Color::Rgb(136, 57, 239),
        red: Color::Rgb(210, 15, 57),
        green: Color::Rgb(64, 160, 43),
        yellow: Color::Rgb(223, 142, 29),
        blue: Color::Rgb(30, 102, 245),
        lavender: Color::Rgb(114, 135, 253),
        peach: Color::Rgb(254, 100, 11),
        teal: Color::Rgb(23, 146, 153),
    },
];

fn get_theme(index: usize) -> &'static Theme {
    &THEMES[index % THEMES.len()]
}

fn border_style(focused: bool, t: &Theme) -> Style {
    if focused {
        Style::default().fg(t.accent)
    } else {
        Style::default().fg(t.overlay0)
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    let t = get_theme(app.theme_index);

    let query_height = {
        let line_count = app.query_input.matches('\n').count() + 1;
        (line_count as u16 + 2).clamp(3, 8)
    };

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(query_height),
            Constraint::Length(1),
        ])
        .split(size);

    draw_title_bar(f, app, outer[0], t);
    draw_main_area(f, app, outer[1], t);
    draw_query_editor(f, app, outer[2], t);
    draw_status_bar(f, app, outer[3], t);

    app.query_editor_area = Some(outer[2]);

    if app.completion.is_some() {
        draw_completion_popup(f, app, outer[2], t);
    }

    if app.show_help {
        draw_help_popup(f, app, size, t);
    }

    if app.show_row_detail {
        draw_row_detail(f, app, size, t);
    }

    if app.show_schema {
        draw_schema_popup(f, app, size, t);
    }

    if app.show_saved_queries {
        draw_saved_queries_popup(f, app, size, t);
    }
}

fn draw_title_bar(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let kind = app.db.kind_name();
    let path = &app.db_path;
    let theme_name = get_theme(app.theme_index).name;
    let brand = " qry ";
    let left = Span::styled(
        brand,
        Style::default()
            .fg(t.crust)
            .bg(t.accent)
            .add_modifier(Modifier::BOLD),
    );
    let right_text = format!(" {kind}: {path}  [{theme_name}] ");
    let right = Span::styled(&right_text, Style::default().fg(t.subtext0));
    let spacer_len = (area.width as usize)
        .saturating_sub(brand.len())
        .saturating_sub(right_text.len());
    let spacer = Span::styled(" ".repeat(spacer_len), Style::default().bg(t.mantle));

    let line = Line::from(vec![left, spacer, right]);
    let p = Paragraph::new(line).style(Style::default().bg(t.mantle));
    f.render_widget(p, area);
}

fn draw_main_area(f: &mut Frame, app: &mut App, area: Rect, t: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(app.pane_width), Constraint::Min(30)])
        .split(area);

    app.table_list_area = Some(chunks[0]);
    app.data_view_area = Some(chunks[1]);

    draw_table_list(f, app, chunks[0], t);
    draw_data_view(f, app, chunks[1], t);
}

fn draw_table_list(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let focused = app.focus == Focus::Tables;
    let items: Vec<ListItem> = app
        .tables
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_view = i >= app.view_start_index;
            let style = if i == app.selected_table {
                Style::default()
                    .fg(t.accent)
                    .bg(t.surface0)
                    .add_modifier(Modifier::BOLD)
            } else if is_view {
                Style::default().fg(t.teal)
            } else {
                Style::default().fg(t.text)
            };
            let prefix = if i == app.selected_table {
                "▸ "
            } else {
                "  "
            };
            let suffix = if is_view { " (view)" } else { "" };
            ListItem::new(format!("{prefix}{name}{suffix}")).style(style)
        })
        .collect();

    let view_count = app.tables.len() - app.view_start_index;
    let table_count = app.view_start_index;
    let title = if view_count > 0 {
        format!(" Tables ({table_count}) Views ({view_count}) ")
    } else {
        format!(" Tables ({table_count}) ")
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused, t));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_data_view(f: &mut Frame, app: &mut App, area: Rect, t: &Theme) {
    let focused = app.focus == Focus::Data;
    let title = build_data_title(app);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style(focused, t));

    if app.table_data.is_none() {
        let msg = Paragraph::new("  No data to display")
            .style(Style::default().fg(t.overlay0))
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    if app.table_data.as_ref().unwrap().columns.is_empty() {
        let msg = Paragraph::new("  Empty result set")
            .style(Style::default().fg(t.overlay0))
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let inner = block.inner(area);
    f.render_widget(block, area);

    let visible_height = inner.height.saturating_sub(2) as usize;
    app.ensure_cursor_visible(visible_height);

    {
        let data = app.table_data.as_ref().unwrap();
        let visible_cols = compute_visible_columns(data, inner.width as usize, app.data_scroll_col, app);
        if !visible_cols.is_empty() {
            let last_visible = visible_cols.last().unwrap().0;
            let first_visible = visible_cols.first().unwrap().0;
            if app.selected_col > last_visible {
                app.data_scroll_col += app.selected_col - last_visible;
            } else if app.selected_col < first_visible {
                app.data_scroll_col = app.selected_col;
            }
        }
    }

    let data = app.table_data.as_ref().unwrap();
    let visible_cols = compute_visible_columns(data, inner.width as usize, app.data_scroll_col, app);
    let col_range = &visible_cols;

    let search_lower = if !app.search_query.is_empty() {
        Some(app.search_query.to_lowercase())
    } else {
        None
    };

    let editing = app.input_mode == InputMode::EditCell;

    let header_cells: Vec<Cell> = col_range
        .iter()
        .map(|&(ci, w)| {
            let name = &data.columns[ci];
            let pin_marker = if ci < app.pinned_columns { "📌" } else { "" };
            let sort_indicator = if app.sort_column == Some(ci) {
                if app.sort_ascending {
                    " ▲"
                } else {
                    " ▼"
                }
            } else {
                ""
            };
            let display = format!(
                "{}{}{}",
                pin_marker,
                truncate_str(name, w.saturating_sub(sort_indicator.len()).saturating_sub(pin_marker.len())),
                sort_indicator
            );
            let mut style = Style::default().fg(t.accent).add_modifier(Modifier::BOLD);
            if app.selected_col == ci && focused {
                style = style.bg(t.surface1);
            }
            Cell::from(display).style(style)
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    let max_scroll = data.rows.len().saturating_sub(visible_height);
    let scroll = app.data_scroll_row.min(max_scroll);

    let rows: Vec<Row> = data
        .rows
        .iter()
        .skip(scroll)
        .take(visible_height)
        .enumerate()
        .map(|(vi, row)| {
            let abs_row = vi + scroll;
            let is_selected = abs_row == app.selected_row;

            let cells: Vec<Cell> = col_range
                .iter()
                .map(|&(ci, w)| {
                    let raw = row.get(ci).map(|s| s.as_str()).unwrap_or("");

                    if editing && is_selected && ci == app.selected_col && focused {
                        let display = format!("{}▎", truncate_str(&app.edit_buffer, w.saturating_sub(1)));
                        return Cell::from(display).style(
                            Style::default()
                                .fg(t.crust)
                                .bg(t.yellow)
                                .add_modifier(Modifier::BOLD),
                        );
                    }

                    let val = app.format_cell_value(raw);
                    let truncated = truncate_str(&val, w);

                    let is_null = raw == "NULL";
                    let is_empty = raw.is_empty();
                    let is_blob = raw.starts_with("<blob ");
                    let is_search_match = search_lower
                        .as_ref()
                        .map(|q| raw.to_lowercase().contains(q))
                        .unwrap_or(false);
                    let is_selected_cell = is_selected && ci == app.selected_col && focused;

                    let style = if is_selected_cell {
                        Style::default().fg(t.crust).bg(t.accent)
                    } else if is_search_match {
                        Style::default().fg(t.crust).bg(t.yellow)
                    } else if is_null {
                        Style::default()
                            .fg(t.overlay0)
                            .add_modifier(Modifier::ITALIC)
                    } else if is_empty {
                        Style::default().fg(t.surface1)
                    } else if is_blob {
                        Style::default().fg(t.peach)
                    } else {
                        Style::default()
                    };

                    Cell::from(truncated).style(style)
                })
                .collect();

            let base_style = if is_selected && focused {
                Style::default().fg(t.text).bg(t.surface0)
            } else if abs_row % 2 == 0 {
                Style::default().fg(t.text)
            } else {
                Style::default().fg(t.text).bg(t.mantle)
            };
            Row::new(cells).style(base_style)
        })
        .collect();

    let widths: Vec<Constraint> = col_range
        .iter()
        .map(|&(_, w)| Constraint::Length(w as u16))
        .collect();

    let table = Table::new(rows, &widths).header(header);

    f.render_widget(table, inner);

    if data.rows.len() > visible_height {
        draw_scrollbar(f, inner, scroll, data.rows.len(), visible_height, t);
    }
}

fn build_data_title(app: &App) -> String {
    let base = match app.data_source {
        DataSource::Table if !app.tables.is_empty() => {
            app.tables[app.selected_table].clone()
        }
        DataSource::Query => "Query Results".to_string(),
        _ => "Data".to_string(),
    };

    let filter_marker = if !app.filter_query.is_empty() {
        format!(" [filter: {}]", app.filter_query)
    } else {
        String::new()
    };

    if let Some(ref data) = app.table_data {
        let total = app.total_row_count.unwrap_or(data.rows.len());
        let showing = data.rows.len();
        let row_pos = app.selected_row + 1;
        if total > showing {
            format!(" {base} (row {row_pos}/{showing} of {total}){filter_marker} ")
        } else {
            format!(" {base} (row {row_pos}/{total}){filter_marker} ")
        }
    } else {
        format!(" {base}{filter_marker} ")
    }
}

fn compute_visible_columns(
    data: &crate::db::QueryResult,
    available_width: usize,
    scroll_col: usize,
    app: &App,
) -> Vec<(usize, usize)> {
    let col_widths: Vec<usize> = data
        .columns
        .iter()
        .enumerate()
        .map(|(ci, col_name)| {
            let header_w = col_name.len();
            let max_data_w = data
                .rows
                .iter()
                .take(200)
                .map(|row| row.get(ci).map(|s| s.len()).unwrap_or(0))
                .max()
                .unwrap_or(0);
            let base = header_w.max(max_data_w).clamp(4, 40) + 1;
            let adj = app.column_adj(ci);
            (base as i16 + adj).max(4) as usize
        })
        .collect();

    let mut visible = Vec::new();
    let mut used = 0usize;

    for ci in 0..app.pinned_columns.min(col_widths.len()) {
        let w = col_widths[ci];
        if used + w > available_width && !visible.is_empty() {
            break;
        }
        visible.push((ci, w));
        used += w;
    }

    let start = scroll_col.max(app.pinned_columns).min(col_widths.len().saturating_sub(1));

    for (i, &w) in col_widths.iter().enumerate().skip(start) {
        if i < app.pinned_columns {
            continue;
        }
        if used + w > available_width && !visible.is_empty() {
            break;
        }
        visible.push((i, w));
        used += w;
    }

    if visible.is_empty() && !col_widths.is_empty() {
        visible.push((start, available_width));
    }

    visible
}

fn draw_scrollbar(f: &mut Frame, area: Rect, offset: usize, total: usize, visible: usize, t: &Theme) {
    if total == 0 || area.height < 3 {
        return;
    }
    let bar_area = Rect::new(area.x + area.width - 1, area.y + 1, 1, area.height - 1);
    let track_h = bar_area.height as usize;
    if track_h == 0 {
        return;
    }
    let thumb_h = ((visible as f64 / total as f64) * track_h as f64)
        .ceil()
        .max(1.0) as usize;
    let max_offset = total.saturating_sub(visible);
    let thumb_pos = if max_offset > 0 {
        ((offset as f64 / max_offset as f64) * (track_h - thumb_h) as f64).round() as usize
    } else {
        0
    };

    for y in 0..track_h {
        let ch = if y >= thumb_pos && y < thumb_pos + thumb_h {
            "█"
        } else {
            "░"
        };
        let style = if y >= thumb_pos && y < thumb_pos + thumb_h {
            Style::default().fg(t.lavender)
        } else {
            Style::default().fg(t.surface1)
        };
        let buf = f.buffer_mut();
        let x = bar_area.x;
        let row = bar_area.y + y as u16;
        if row < bar_area.y + bar_area.height {
            buf[(x, row)].set_symbol(ch);
            buf[(x, row)].set_style(style);
        }
    }
}

fn draw_query_editor(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let focused = app.focus == Focus::QueryEditor;
    let title = if app.query_error.is_some() {
        " SQL Query (error!) ".to_string()
    } else if !app.query_history.is_empty() {
        if let Some(idx) = app.history_index {
            format!(" SQL Query [{}/{}] ", idx + 1, app.query_history.len())
        } else {
            format!(" SQL Query (history: {}) ", app.query_history.len())
        }
    } else {
        " SQL Query (Enter to run, Alt+Enter for newline) ".to_string()
    };

    draw_query_editor_with_title(f, app, area, &title, focused, t);
}

fn draw_query_editor_with_title(
    f: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    focused: bool,
    t: &Theme,
) {
    let style = if app.query_error.is_some() {
        Style::default().fg(t.red)
    } else {
        border_style(focused, t)
    };

    let block = Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_style(style);

    if app.query_input.is_empty() && !focused {
        let display_text = Span::styled(
            "Type a SQL query here...",
            Style::default()
                .fg(t.overlay0)
                .add_modifier(Modifier::ITALIC),
        );
        let paragraph = Paragraph::new(Line::from(display_text))
            .block(block)
            .style(Style::default());
        f.render_widget(paragraph, area);
        return;
    }

    let lines = highlight_sql(&app.query_input, app.query_error_token.as_deref(), t);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default());
    f.render_widget(paragraph, area);

    if focused {
        let inner = area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });
        let (cx, cy) = cursor_position_multiline(&app.query_input, app.query_cursor);
        let x = inner.x + cx;
        let y = inner.y + cy;
        if x < inner.x + inner.width && y < inner.y + inner.height {
            f.set_cursor_position((x, y));
        }
    }
}

fn cursor_position_multiline(input: &str, byte_cursor: usize) -> (u16, u16) {
    let before = &input[..byte_cursor];
    let lines: Vec<&str> = before.split('\n').collect();
    let row = lines.len() - 1;
    let col = lines.last().map(|l| l.chars().count()).unwrap_or(0);
    (col as u16, row as u16)
}

fn highlight_sql<'a>(input: &'a str, error_token: Option<&str>, t: &Theme) -> Vec<Line<'a>> {
    let keywords: &[&str] = &[
        "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
        "TABLE", "INDEX", "INTO", "VALUES", "SET", "AND", "OR", "NOT", "NULL", "IS", "IN",
        "LIKE", "BETWEEN", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "FULL", "CROSS", "ON",
        "AS", "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "UNION", "DISTINCT", "COUNT",
        "SUM", "MAX", "MIN", "ASC", "DESC", "CASE", "WHEN", "THEN", "ELSE", "END", "EXISTS",
        "CAST", "WITH", "IF", "PRIMARY", "EXPLAIN", "TRUE", "FALSE", "USING", "AVG", "COALESCE",
    ];

    input
        .split('\n')
        .map(|line_str| {
            let mut spans: Vec<Span> = Vec::new();
            let chars = line_str.char_indices().peekable();
            let mut pos = 0;

            while pos < line_str.len() {
                let ch = line_str[pos..].chars().next().unwrap();

                if ch == '\'' {
                    let start = pos;
                    pos += 1;
                    while pos < line_str.len() {
                        let c = line_str[pos..].chars().next().unwrap();
                        pos += c.len_utf8();
                        if c == '\'' {
                            break;
                        }
                    }
                    spans.push(Span::styled(
                        &line_str[start..pos],
                        Style::default().fg(t.green),
                    ));
                    continue;
                }

                if ch == '-' && pos + 1 < line_str.len() && line_str.as_bytes().get(pos + 1) == Some(&b'-') {
                    spans.push(Span::styled(
                        &line_str[pos..],
                        Style::default().fg(t.overlay0).add_modifier(Modifier::ITALIC),
                    ));
                    pos = line_str.len();
                    continue;
                }

                if ch.is_ascii_digit()
                    || (ch == '-'
                        && pos + 1 < line_str.len()
                        && line_str[pos + 1..]
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit())
                            .unwrap_or(false))
                {
                    let start = pos;
                    if ch == '-' {
                        pos += 1;
                    }
                    while pos < line_str.len()
                        && line_str[pos..]
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit() || c == '.')
                            .unwrap_or(false)
                    {
                        pos += line_str[pos..].chars().next().unwrap().len_utf8();
                    }
                    spans.push(Span::styled(
                        &line_str[start..pos],
                        Style::default().fg(t.peach),
                    ));
                    continue;
                }

                if ch.is_alphanumeric() || ch == '_' {
                    let start = pos;
                    while pos < line_str.len()
                        && line_str[pos..]
                            .chars()
                            .next()
                            .map(|c| c.is_alphanumeric() || c == '_')
                            .unwrap_or(false)
                    {
                        pos += line_str[pos..].chars().next().unwrap().len_utf8();
                    }
                    let word = &line_str[start..pos];
                    let upper = word.to_uppercase();

                    let is_error = error_token
                        .map(|tok| tok.eq_ignore_ascii_case(word))
                        .unwrap_or(false);

                    if is_error {
                        spans.push(Span::styled(
                            word,
                            Style::default()
                                .fg(t.red)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        ));
                    } else if keywords.iter().any(|&kw| kw == upper) {
                        spans.push(Span::styled(
                            word,
                            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        spans.push(Span::styled(word, Style::default().fg(t.text)));
                    }
                    continue;
                }

                if "(),;.*=<>!+-/".contains(ch) {
                    spans.push(Span::styled(
                        &line_str[pos..pos + ch.len_utf8()],
                        Style::default().fg(t.blue),
                    ));
                    pos += ch.len_utf8();
                    continue;
                }

                spans.push(Span::raw(&line_str[pos..pos + ch.len_utf8()]));
                pos += ch.len_utf8();
            }

            drop(chars);
            Line::from(spans)
        })
        .collect()
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    match app.input_mode {
        InputMode::Search => {
            draw_input_bar(f, " / ", &app.search_query, "Enter to search, Esc to cancel", t, area);
            return;
        }
        InputMode::Filter => {
            draw_input_bar(f, " F ", &app.filter_query, "Enter WHERE clause, Esc to cancel", t, area);
            return;
        }
        InputMode::Export => {
            draw_input_bar(f, " E ", &app.export_input, "Enter to export, Esc to cancel", t, area);
            return;
        }
        InputMode::SaveQueryName => {
            draw_input_bar(f, " S ", &app.save_query_name_input, "Enter name, Esc to cancel", t, area);
            return;
        }
        InputMode::EditCell => {
            draw_input_bar(f, " EDIT ", &app.edit_buffer, "Enter to save, Esc to cancel", t, area);
            return;
        }
        InputMode::DeleteConfirm => {
            let line = Line::from(vec![
                Span::styled(" DEL ", Style::default().fg(t.crust).bg(t.red).add_modifier(Modifier::BOLD)),
                Span::styled(" Delete this row? (y/n) ", Style::default().fg(t.text)),
            ]);
            let p = Paragraph::new(line).style(Style::default().bg(t.crust));
            f.render_widget(p, area);
            return;
        }
        InputMode::Normal => {}
    }

    let help_hint = " ?:help Tab:focus q:quit ";
    let mut indicators = Vec::new();
    if app.hex_mode {
        indicators.push("[HEX]");
    }
    if app.pinned_columns > 0 {
        indicators.push("[PIN]");
    }
    if !app.filter_query.is_empty() {
        indicators.push("[FILTER]");
    }
    let indicator_str = if indicators.is_empty() {
        String::new()
    } else {
        format!(" {}", indicators.join(" "))
    };

    let timing = app
        .query_duration
        .map(|d| format!(" [{}]", crate::app::format_duration(d)))
        .unwrap_or_default();

    let left_text = format!("{}{}{}", app.status_msg, timing, indicator_str);
    let left = Span::styled(&left_text, Style::default().fg(t.text));
    let right_len = help_hint.len();
    let spacer_len = (area.width as usize)
        .saturating_sub(left_text.len())
        .saturating_sub(right_len);
    let spacer = Span::raw(" ".repeat(spacer_len));
    let right = Span::styled(help_hint, Style::default().fg(t.subtext0));

    let line = Line::from(vec![left, spacer, right]);
    let p = Paragraph::new(line).style(Style::default().bg(t.crust));
    f.render_widget(p, area);
}

fn draw_input_bar(f: &mut Frame, label: &str, value: &str, hint: &str, t: &Theme, area: Rect) {
    let label_span = Span::styled(
        label,
        Style::default()
            .fg(t.crust)
            .bg(t.yellow)
            .add_modifier(Modifier::BOLD),
    );
    let value_span = Span::styled(value, Style::default().fg(t.text));
    let cursor = Span::styled("▎", Style::default().fg(t.accent));
    let hint_span = Span::styled(format!("  {hint}"), Style::default().fg(t.overlay0));

    let line = Line::from(vec![label_span, value_span, cursor, hint_span]);
    let p = Paragraph::new(line).style(Style::default().bg(t.crust));
    f.render_widget(p, area);
}

fn draw_completion_popup(f: &mut Frame, app: &App, editor_area: Rect, t: &Theme) {
    let Some(ref state) = app.completion else {
        return;
    };

    let max_visible = 8usize.min(state.candidates.len());
    let popup_h = max_visible as u16 + 2;
    let max_text_w = state
        .candidates
        .iter()
        .map(|c| c.text.len() + 4)
        .max()
        .unwrap_or(10);
    let popup_w = (max_text_w as u16 + 2).clamp(14, 40);

    let char_offset = app.query_input[..state.prefix_start].chars().count() as u16;
    let x = (editor_area.x + 1 + char_offset).min(editor_area.x + editor_area.width - popup_w);
    let y = editor_area.y.saturating_sub(popup_h);

    let popup_area = Rect::new(x, y, popup_w, popup_h);
    f.render_widget(Clear, popup_area);

    let scroll_top = if state.index >= max_visible {
        state.index - max_visible + 1
    } else {
        0
    };

    let items: Vec<ListItem> = state
        .candidates
        .iter()
        .enumerate()
        .skip(scroll_top)
        .take(max_visible)
        .map(|(i, item)| {
            let (icon, icon_color) = match item.kind {
                CompletionKind::Keyword => ("K", t.accent),
                CompletionKind::Table => ("T", t.green),
                CompletionKind::Column => ("C", t.peach),
            };
            if i == state.index {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{icon} "),
                        Style::default()
                            .fg(t.crust)
                            .bg(t.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &item.text,
                        Style::default()
                            .fg(t.crust)
                            .bg(t.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                    Span::styled(&item.text, Style::default().fg(t.text)),
                ]))
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.surface1))
        .style(Style::default().bg(t.surface0));

    let list = List::new(items).block(block);
    f.render_widget(list, popup_area);
}

fn draw_help_popup(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let popup_w = 62u16.min(area.width.saturating_sub(4));
    let popup_h = 48u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup_area);

    let ks = Style::default().fg(t.lavender);
    let ds = Style::default().fg(t.text);
    let help_text = vec![
        Line::from(Span::styled("Keyboard Shortcuts", Style::default().fg(t.accent).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Data View", Style::default().fg(t.peach).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  ↑/↓ j/k           ", ks), Span::styled("Move row cursor", ds)]),
        Line::from(vec![Span::styled("  ←/→ h/l           ", ks), Span::styled("Move column cursor", ds)]),
        Line::from(vec![Span::styled("  PgUp / PgDn       ", ks), Span::styled("Page up/down", ds)]),
        Line::from(vec![Span::styled("  g / G             ", ks), Span::styled("First / last row", ds)]),
        Line::from(vec![Span::styled("  Enter             ", ks), Span::styled("Row detail view", ds)]),
        Line::from(vec![Span::styled("  s                 ", ks), Span::styled("Sort by column", ds)]),
        Line::from(vec![Span::styled("  /                 ", ks), Span::styled("Search in data", ds)]),
        Line::from(vec![Span::styled("  n / N             ", ks), Span::styled("Next/prev search match", ds)]),
        Line::from(vec![Span::styled("  f                 ", ks), Span::styled("Filter (WHERE clause)", ds)]),
        Line::from(vec![Span::styled("  y / Y             ", ks), Span::styled("Copy cell / copy row", ds)]),
        Line::from(vec![Span::styled("  Ctrl+Y            ", ks), Span::styled("Copy column", ds)]),
        Line::from(vec![Span::styled("  e / E             ", ks), Span::styled("Export CSV / JSON", ds)]),
        Line::from(vec![Span::styled("  d                 ", ks), Span::styled("Schema / DDL view", ds)]),
        Line::from(vec![Span::styled("  x                 ", ks), Span::styled("Toggle hex display", ds)]),
        Line::from(vec![Span::styled("  + / -             ", ks), Span::styled("Widen / narrow column", ds)]),
        Line::from(vec![Span::styled("  p                 ", ks), Span::styled("Pin/unpin columns", ds)]),
        Line::from(vec![Span::styled("  i                 ", ks), Span::styled("Edit cell (tables only)", ds)]),
        Line::from(vec![Span::styled("  D                 ", ks), Span::styled("Delete row (tables only)", ds)]),
        Line::from(vec![Span::styled("  T                 ", ks), Span::styled("Cycle color theme", ds)]),
        Line::from(""),
        Line::from(Span::styled("Query Editor", Style::default().fg(t.peach).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  Enter             ", ks), Span::styled("Execute query", ds)]),
        Line::from(vec![Span::styled("  Shift+Enter       ", ks), Span::styled("New line", ds)]),
        Line::from(vec![Span::styled("  ↑ / ↓             ", ks), Span::styled("Query history", ds)]),
        Line::from(vec![Span::styled("  Tab / Shift+Tab   ", ks), Span::styled("Fuzzy autocomplete", ds)]),
        Line::from(vec![Span::styled("  Ctrl+Z / Ctrl+Y   ", ks), Span::styled("Undo / redo", ds)]),
        Line::from(vec![Span::styled("  Ctrl+S            ", ks), Span::styled("Save query", ds)]),
        Line::from(vec![Span::styled("  Ctrl+O            ", ks), Span::styled("Open saved queries", ds)]),
        Line::from(vec![Span::styled("  Ctrl+U            ", ks), Span::styled("Clear query", ds)]),
        Line::from(""),
        Line::from(Span::styled("Tables", Style::default().fg(t.peach).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  ↑/↓ j/k           ", ks), Span::styled("Navigate tables", ds)]),
        Line::from(vec![Span::styled("  Enter / l          ", ks), Span::styled("Select table", ds)]),
        Line::from(vec![Span::styled("  d                 ", ks), Span::styled("Schema / DDL view", ds)]),
        Line::from(vec![Span::styled("  r                 ", ks), Span::styled("Refresh", ds)]),
        Line::from(vec![Span::styled("  > / <             ", ks), Span::styled("Resize table pane", ds)]),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(t.peach).add_modifier(Modifier::BOLD))),
        Line::from(vec![Span::styled("  Tab / Shift+Tab    ", ks), Span::styled("Cycle focus", ds)]),
        Line::from(vec![Span::styled("  ? / F1            ", ks), Span::styled("Toggle this help", ds)]),
        Line::from(vec![Span::styled("  q / Ctrl+C        ", ks), Span::styled("Quit", ds)]),
        Line::from(vec![Span::styled("  Mouse             ", ks), Span::styled("Click & scroll support", ds)]),
    ];

    let total_lines = help_text.len();
    let inner_h = popup_h.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(inner_h);
    let scroll = app.help_scroll.min(max_scroll);

    let block = Block::default()
        .title(Span::styled(
            " Help ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " ↑/↓ scroll  Esc/q close ",
            Style::default().fg(t.overlay0),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.surface1))
        .style(Style::default().bg(t.base));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .scroll((scroll as u16, 0));
    f.render_widget(paragraph, popup_area);
}

fn draw_row_detail(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let Some(ref data) = app.table_data else {
        return;
    };
    let Some(row) = data.rows.get(app.selected_row) else {
        return;
    };

    let popup_w = (area.width * 3 / 4).max(40).min(area.width.saturating_sub(4));
    let popup_h = (area.height * 3 / 4).max(10).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup_area);

    let title = format!(" Row {} Detail ", app.selected_row + 1);

    let max_col_name_len = data.columns.iter().map(|c| c.len()).max().unwrap_or(8);
    let name_width = max_col_name_len.min(20);

    let visible_inner_h = popup_h.saturating_sub(2) as usize;
    let total_lines = data.columns.len();
    let scroll = app
        .detail_scroll
        .min(total_lines.saturating_sub(visible_inner_h));

    let mut lines: Vec<Line> = Vec::new();
    for (_i, (col, val)) in data
        .columns
        .iter()
        .zip(row.iter())
        .enumerate()
        .skip(scroll)
        .take(visible_inner_h)
    {
        let col_display = format!("{:>width$}", col, width = name_width);
        let is_null = val == "NULL";
        let is_empty = val.is_empty();

        let val_style = if is_null {
            Style::default()
                .fg(t.overlay0)
                .add_modifier(Modifier::ITALIC)
        } else if is_empty {
            Style::default().fg(t.surface1)
        } else {
            Style::default().fg(t.text)
        };

        let display_val = if is_null {
            "NULL"
        } else if is_empty {
            "(empty)"
        } else {
            val
        };

        lines.push(Line::from(vec![
            Span::styled(
                col_display,
                Style::default()
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  │ ", Style::default().fg(t.surface1)),
            Span::styled(display_val.to_string(), val_style),
        ]));
    }

    let nav_hint = format!(
        " ↑/↓ scroll  ←/→ prev/next row  Esc close  [{}/{}] ",
        app.selected_row + 1,
        data.rows.len()
    );

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            nav_hint,
            Style::default().fg(t.overlay0),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.surface1))
        .style(Style::default().bg(t.base));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_schema_popup(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let Some(ref schema) = app.schema_info else {
        return;
    };

    let popup_w = (area.width * 3 / 4).max(50).min(area.width.saturating_sub(4));
    let popup_h = (area.height * 3 / 4).max(12).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup_area);

    let title = format!(" Schema: {} ", schema.table_name);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        "CREATE Statement",
        Style::default().fg(t.peach).add_modifier(Modifier::BOLD),
    )));
    for ddl_line in schema.create_sql.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {ddl_line}"),
            Style::default().fg(t.green),
        )));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "Columns",
        Style::default().fg(t.peach).add_modifier(Modifier::BOLD),
    )));
    for col in &schema.columns {
        let pk_marker = if col.is_pk { " PK" } else { "   " };
        let nn_marker = if col.notnull { " NOT NULL" } else { "" };
        let default = if !col.default_value.is_empty() {
            format!(" DEFAULT {}", col.default_value)
        } else {
            String::new()
        };
        lines.push(Line::from(vec![
            Span::styled(
                pk_marker,
                Style::default()
                    .fg(t.yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" {:<20}", col.name), Style::default().fg(t.accent)),
            Span::styled(format!(" {:<15}", col.col_type), Style::default().fg(t.text)),
            Span::styled(nn_marker.to_string(), Style::default().fg(t.red)),
            Span::styled(default, Style::default().fg(t.overlay0)),
        ]));
    }

    if !schema.indexes.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Indexes",
            Style::default().fg(t.peach).add_modifier(Modifier::BOLD),
        )));
        for idx in &schema.indexes {
            let unique = if idx.unique { " UNIQUE" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}", idx.name), Style::default().fg(t.lavender)),
                Span::styled(unique.to_string(), Style::default().fg(t.yellow)),
            ]));
        }
    }

    if !schema.foreign_keys.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Foreign Keys",
            Style::default().fg(t.peach).add_modifier(Modifier::BOLD),
        )));
        for fk in &schema.foreign_keys {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", fk.from_column), Style::default().fg(t.accent)),
                Span::styled("→ ", Style::default().fg(t.overlay0)),
                Span::styled(&fk.to_table, Style::default().fg(t.green)),
                Span::styled(format!(".{}", fk.to_column), Style::default().fg(t.teal)),
            ]));
        }
    }

    let visible_inner_h = popup_h.saturating_sub(2) as usize;
    let total_lines = lines.len();
    let scroll = app
        .schema_scroll
        .min(total_lines.saturating_sub(visible_inner_h));

    let visible_lines: Vec<Line> = lines.into_iter().skip(scroll).take(visible_inner_h).collect();

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " ↑/↓ scroll  Esc/q close ",
            Style::default().fg(t.overlay0),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.surface1))
        .style(Style::default().bg(t.base));

    let paragraph = Paragraph::new(visible_lines).block(block);
    f.render_widget(paragraph, popup_area);
}

fn draw_saved_queries_popup(f: &mut Frame, app: &App, area: Rect, t: &Theme) {
    let popup_w = (area.width * 2 / 3).max(40).min(area.width.saturating_sub(4));
    let popup_h = (area.height / 2).max(8).min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup_area);

    if app.saved_queries.is_empty() {
        let block = Block::default()
            .title(Span::styled(
                " Saved Queries ",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.surface1))
            .style(Style::default().bg(t.base));
        let msg = Paragraph::new("  No saved queries. Use Ctrl+S to save.")
            .style(Style::default().fg(t.overlay0))
            .block(block);
        f.render_widget(msg, popup_area);
        return;
    }

    let inner_w = popup_w.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .saved_queries
        .iter()
        .enumerate()
        .map(|(i, (name, sql))| {
            let preview: String = sql.chars().take(inner_w.saturating_sub(name.len() + 4)).collect();
            let preview = preview.replace('\n', " ");
            if i == app.saved_query_selected {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("▸ {name}"),
                        Style::default()
                            .fg(t.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {preview}"), Style::default().fg(t.subtext0)),
                ]))
                .style(Style::default().bg(t.surface0))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {name}"), Style::default().fg(t.text)),
                    Span::styled(format!("  {preview}"), Style::default().fg(t.overlay0)),
                ]))
            }
        })
        .collect();

    let block = Block::default()
        .title(Span::styled(
            " Saved Queries ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " Enter:load  d:delete  Esc:close ",
            Style::default().fg(t.overlay0),
        )))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.surface1))
        .style(Style::default().bg(t.base));

    let list = List::new(items).block(block);
    f.render_widget(list, popup_area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        format!("{:<width$}", s, width = max)
    } else if max > 1 {
        format!("{}…", &s[..max - 1])
    } else {
        "…".to_string()
    }
}

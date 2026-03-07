use crate::app::{App, Focus, InputMode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::time::Duration;

pub fn poll_event() -> std::io::Result<Option<Event>> {
    if event::poll(Duration::from_millis(50))? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if app.show_help {
        match key.code {
            KeyCode::Char('?') | KeyCode::F(1) | KeyCode::Esc | KeyCode::Char('q') => {
                app.show_help = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll += 1;
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                app.help_scroll += 10;
            }
            KeyCode::Char('g') => {
                app.help_scroll = 0;
            }
            KeyCode::Char('G') => {
                app.help_scroll = usize::MAX;
            }
            _ => {}
        }
        return;
    }

    if app.show_saved_queries {
        handle_saved_queries(app, key);
        return;
    }

    if app.show_row_detail {
        handle_row_detail(app, key);
        return;
    }

    if app.show_schema {
        handle_schema(app, key);
        return;
    }

    match app.input_mode {
        InputMode::Search => {
            handle_search_input(app, key);
            return;
        }
        InputMode::Filter => {
            handle_filter_input(app, key);
            return;
        }
        InputMode::Export => {
            handle_export_input(app, key);
            return;
        }
        InputMode::SaveQueryName => {
            handle_save_query_input(app, key);
            return;
        }
        InputMode::EditCell => {
            handle_edit_input(app, key);
            return;
        }
        InputMode::DeleteConfirm => {
            handle_delete_confirm(app, key);
            return;
        }
        InputMode::Normal => {}
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key.code {
            app.running = false;
            return;
        }
    }

    match app.focus {
        Focus::QueryEditor => handle_query_editor(app, key),
        Focus::Tables => handle_tables(app, key),
        Focus::Data => handle_data(app, key),
    }
}

pub fn handle_mouse(app: &mut App, kind: MouseEventKind, col: u16, row: u16) {
    if app.show_help || app.show_row_detail || app.show_schema || app.show_saved_queries {
        return;
    }
    if app.input_mode != InputMode::Normal {
        return;
    }
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            app.click_at(col, row);
        }
        MouseEventKind::ScrollDown => {
            app.scroll_at(col, row, true);
        }
        MouseEventKind::ScrollUp => {
            app.scroll_at(col, row, false);
        }
        _ => {}
    }
}

fn handle_common(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Tab => {
            app.cycle_focus_forward();
            true
        }
        KeyCode::BackTab => {
            app.cycle_focus_backward();
            true
        }
        KeyCode::Char('?') | KeyCode::F(1) => {
            app.show_help = true;
            app.help_scroll = 0;
            true
        }
        KeyCode::Char('q') => {
            app.running = false;
            true
        }
        _ => false,
    }
}

fn handle_tables(app: &mut App, key: KeyEvent) {
    if handle_common(app, key) {
        return;
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.select_table_up(),
        KeyCode::Down | KeyCode::Char('j') => app.select_table_down(),
        KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
            app.load_selected_table();
            app.focus = Focus::Data;
        }
        KeyCode::Char('r') => app.refresh_tables(),
        KeyCode::Char('d') => app.load_schema(),
        KeyCode::Char('>') | KeyCode::Char('.') => app.grow_pane(),
        KeyCode::Char('<') | KeyCode::Char(',') => app.shrink_pane(),
        KeyCode::Char('T') => app.cycle_theme(),
        _ => {}
    }
}

fn handle_data(app: &mut App, key: KeyEvent) {
    if handle_common(app, key) {
        return;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('y') => {
                app.yank_column();
                return;
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => app.scroll_data_up(),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_data_down(),
        KeyCode::Left | KeyCode::Char('h') => app.scroll_data_left(),
        KeyCode::Right | KeyCode::Char('l') => app.scroll_data_right(),
        KeyCode::PageUp => app.page_up(),
        KeyCode::PageDown => app.page_down(),
        KeyCode::Char('g') => app.go_to_first_row(),
        KeyCode::Char('G') => app.go_to_last_row(),
        KeyCode::Enter => app.toggle_row_detail(),
        KeyCode::Char('s') => app.toggle_sort(),
        KeyCode::Char('/') => app.start_search(),
        KeyCode::Char('n') => app.search_next(),
        KeyCode::Char('N') => app.search_prev(),
        KeyCode::Char('y') => app.yank_cell(),
        KeyCode::Char('Y') => app.yank_row(),
        KeyCode::Char('e') => app.start_export_csv(),
        KeyCode::Char('E') => app.start_export_json(),
        KeyCode::Char('d') => app.load_schema(),
        KeyCode::Char('x') => app.toggle_hex_mode(),
        KeyCode::Char('r') => app.refresh_tables(),
        KeyCode::Char('f') => app.start_filter(),
        KeyCode::Char('+') | KeyCode::Char('=') => app.widen_column(),
        KeyCode::Char('-') => app.narrow_column(),
        KeyCode::Char('p') => app.toggle_pin(),
        KeyCode::Char('i') => app.start_edit(),
        KeyCode::Char('D') => app.start_delete_confirm(),
        KeyCode::Char('T') => app.cycle_theme(),
        _ => {}
    }
}

fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Enter => {
            app.end_search();
            app.search_next();
        }
        KeyCode::Backspace => app.search_backspace(),
        KeyCode::Char(c) => app.search_insert_char(c),
        _ => {}
    }
}

fn handle_filter_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_filter(),
        KeyCode::Enter => app.end_filter(),
        KeyCode::Backspace => app.filter_backspace(),
        KeyCode::Char(c) => app.filter_insert_char(c),
        _ => {}
    }
}

fn handle_export_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_export(),
        KeyCode::Enter => app.confirm_export(),
        KeyCode::Backspace => app.export_backspace(),
        KeyCode::Char(c) => app.export_insert_char(c),
        _ => {}
    }
}

fn handle_save_query_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_save_query(),
        KeyCode::Enter => app.confirm_save_query(),
        KeyCode::Backspace => app.save_query_backspace(),
        KeyCode::Char(c) => app.save_query_insert_char(c),
        _ => {}
    }
}

fn handle_edit_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.cancel_edit(),
        KeyCode::Enter => app.confirm_edit(),
        KeyCode::Backspace => app.edit_backspace(),
        KeyCode::Char(c) => app.edit_insert_char(c),
        _ => {}
    }
}

fn handle_delete_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
        _ => app.cancel_delete(),
    }
}

fn handle_saved_queries(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.show_saved_queries = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.saved_query_selected > 0 {
                app.saved_query_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.saved_query_selected < app.saved_queries.len().saturating_sub(1) {
                app.saved_query_selected += 1;
            }
        }
        KeyCode::Enter => app.load_saved_query(),
        KeyCode::Char('d') | KeyCode::Delete => app.delete_saved_query(),
        _ => {}
    }
}

fn handle_row_detail(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
            app.show_row_detail = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.detail_scroll += 1;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.scroll_data_up();
            app.detail_scroll = 0;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.scroll_data_down();
            app.detail_scroll = 0;
        }
        _ => {}
    }
}

fn handle_schema(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('d') => {
            app.show_schema = false;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.schema_scroll = app.schema_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.schema_scroll += 1;
        }
        _ => {}
    }
}

fn handle_query_editor(app: &mut App, key: KeyEvent) {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('u') => {
                app.dismiss_completion();
                app.query_clear();
                return;
            }
            KeyCode::Char('a') => {
                app.dismiss_completion();
                app.query_move_home();
                return;
            }
            KeyCode::Char('e') => {
                app.dismiss_completion();
                app.query_move_end();
                return;
            }
            KeyCode::Char('z') => {
                app.dismiss_completion();
                app.undo();
                return;
            }
            KeyCode::Char('y') => {
                app.dismiss_completion();
                app.redo();
                return;
            }
            KeyCode::Char('s') => {
                app.dismiss_completion();
                app.start_save_query();
                return;
            }
            KeyCode::Char('o') => {
                app.dismiss_completion();
                app.toggle_saved_queries();
                return;
            }
            _ => {}
        }
    }

    if key.code == KeyCode::Enter
        && (key.modifiers.contains(KeyModifiers::SHIFT)
            || key.modifiers.contains(KeyModifiers::ALT))
    {
        app.dismiss_completion();
        app.query_insert_newline();
        return;
    }

    match key.code {
        KeyCode::Tab => {
            if app.completion.is_some() {
                app.cycle_completion();
            } else {
                app.trigger_completion();
            }
        }
        KeyCode::BackTab => {
            if app.completion.is_some() {
                app.cycle_completion_back();
            } else {
                app.dismiss_completion();
                app.cycle_focus_backward();
            }
        }
        KeyCode::Enter => {
            app.dismiss_completion();
            app.run_query();
        }
        KeyCode::Esc => {
            if app.completion.is_some() {
                app.dismiss_completion();
            } else {
                app.focus = Focus::Tables;
            }
        }
        KeyCode::Backspace => {
            app.dismiss_completion();
            app.query_backspace();
        }
        KeyCode::Delete => {
            app.dismiss_completion();
            app.query_delete();
        }
        KeyCode::Left => {
            app.dismiss_completion();
            app.query_move_left();
        }
        KeyCode::Right => {
            app.dismiss_completion();
            app.query_move_right();
        }
        KeyCode::Up => {
            app.dismiss_completion();
            app.history_up();
        }
        KeyCode::Down => {
            app.dismiss_completion();
            app.history_down();
        }
        KeyCode::Home => {
            app.dismiss_completion();
            app.query_move_home();
        }
        KeyCode::End => {
            app.dismiss_completion();
            app.query_move_end();
        }
        KeyCode::Char(c) => {
            app.dismiss_completion();
            app.query_insert_char(c);
        }
        _ => {}
    }
}

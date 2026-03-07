# qry

A fast, feature-rich TUI for exploring SQLite and DuckDB databases.

![Rust](https://img.shields.io/badge/Rust-2021-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

### Database Support

- **SQLite** and **DuckDB** — auto-detected by file magic bytes
- Browse tables and views with row counts
- Inspect schema, DDL, indexes, and foreign key relationships
- Inline cell editing and row deletion with confirmation

### Data Browsing

- Cursor-based navigation with vim keys (`hjkl`) and arrows
- Page up/down, jump to first/last row (`g`/`G`)
- **Sorting** — cycle ascending/descending/clear on any column (`s`)
- **Search** — incremental text search with `n`/`N` navigation (`/`)
- **Filter** — apply a `WHERE` clause to narrow table data (`f`)
- **Row detail popup** — full row view with column names (`Enter`)
- **Hex mode** — display numeric values in hexadecimal (`x`)
- **Column resizing** — widen (`+`) or narrow (`-`) any column
- **Column pinning** — pin left columns so they stay visible while scrolling (`p`)
- **Alternating row colors** and search match highlighting

### Query Editor

- Multi-line SQL editor with **syntax highlighting**
- **Execute queries** with `Enter`, insert newlines with `Shift+Enter`
- **Multi-statement execution** — semicolon-separated statements run in sequence
- **Fuzzy autocompletion** — Tab-triggered for SQL keywords, table names, and column names (subsequence matching with scoring)
- **Query history** — navigate with `↑`/`↓`, persisted across sessions (`~/.local/share/qry/history`)
- **Saved queries** — save (`Ctrl+S`) and load (`Ctrl+O`) named queries, persisted to disk
- **Undo / redo** — `Ctrl+Z` / `Ctrl+Y`
- **Syntax error highlighting** — error tokens underlined in red
- **Query result limit** — custom queries auto-capped at 10,000 rows
- **Query timing** — execution duration shown in status bar

### Export & Clipboard

- **Export to CSV** (`e`) or **JSON** (`E`) with editable filename prompt
- **Copy cell** (`y`), **copy row** (`Y`), or **copy column** (`Ctrl+Y`) to clipboard
- Cross-platform clipboard: `xclip`, `xsel`, `wl-copy` (Linux), `pbcopy` (macOS), `clip.exe` (WSL)

### Schema Inspector

- View **CREATE statement** / DDL
- **Column details** — name, type, primary key, NOT NULL, defaults
- **Indexes** with uniqueness info
- **Foreign keys** — shows `column → table.column` relationships

### Inline Editing

- **Edit cells** — press `i` on any table cell to modify its value in-place
- **Delete rows** — press `D` with `y/n` confirmation
- Edits execute real `UPDATE`/`DELETE` statements against the database

### UI & Theming

- **3 built-in color themes** — Catppuccin Mocha, Tokyo Night, Catppuccin Latte (cycle with `T`)
- **Mouse support** — click to select tables/cells, scroll wheel navigation
- **Resizable panes** — `>` / `<` to adjust the table sidebar width
- **Status bar** — shows query timing, mode indicators (`[HEX]`, `[PIN]`, `[FILTER]`), and help hints
- **Help popup** — press `?` or `F1` for full keybinding reference

## Installation

### From source

```sh
git clone https://github.com/youruser/qry.git
cd qry
cargo build --release
```

The binary will be at `target/release/qry`.

### Requirements

- Rust 2021 edition (1.56+)
- No external database libraries needed — SQLite and DuckDB are bundled

## Usage

```sh
qry path/to/database.db
```

Supported file extensions: `.db`, `.sqlite`, `.sqlite3`, `.duckdb`, `.duck`, `.ddb`

The database type is auto-detected by reading the file header. Files with the SQLite magic bytes are opened as SQLite; everything else is treated as DuckDB.

## Keybindings

### General

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle focus between panels |
| `?` / `F1` | Toggle help popup |
| `q` / `Ctrl+C` | Quit |
| `T` | Cycle color theme |
| Mouse click | Select table/cell/panel |
| Scroll wheel | Scroll in focused panel |

### Tables Panel

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Navigate tables |
| `Enter` / `l` | Select table and focus data |
| `d` | View schema / DDL |
| `r` | Refresh table list |
| `>` / `<` | Resize table pane |

### Data View

| Key | Action |
|-----|--------|
| `↑`/`↓` or `j`/`k` | Move row cursor |
| `←`/`→` or `h`/`l` | Move column cursor |
| `PgUp` / `PgDn` | Page up/down (20 rows) |
| `g` / `G` | Jump to first / last row |
| `Enter` | Open row detail popup |
| `s` | Sort by selected column (asc → desc → clear) |
| `/` | Search in data |
| `n` / `N` | Next / previous search match |
| `f` | Filter (enter a WHERE clause) |
| `y` | Copy cell value to clipboard |
| `Y` | Copy entire row (tab-separated) |
| `Ctrl+Y` | Copy entire column (newline-separated) |
| `e` / `E` | Export to CSV / JSON |
| `d` | View schema / DDL |
| `x` | Toggle hex display |
| `+` / `-` | Widen / narrow selected column |
| `p` | Pin / unpin columns |
| `i` | Edit cell value (tables only) |
| `D` | Delete row with confirmation (tables only) |
| `r` | Refresh data |

### Query Editor

| Key | Action |
|-----|--------|
| `Enter` | Execute query |
| `Shift+Enter` / `Alt+Enter` | Insert newline |
| `↑` / `↓` | Navigate query history |
| `Tab` / `Shift+Tab` | Fuzzy autocomplete / cycle |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Ctrl+S` | Save current query |
| `Ctrl+O` | Open saved queries |
| `Ctrl+U` | Clear query |
| `Ctrl+A` / `Ctrl+E` | Jump to start / end |
| `Esc` | Dismiss completion or leave editor |

## Data Files

qry stores persistent data in `~/.local/share/qry/`:

| File | Contents |
|------|----------|
| `history` | Query history (null-byte separated, last 500 queries) |
| `saved_queries` | Named saved queries |

## Tech Stack

| Crate | Purpose |
|-------|---------|
| [ratatui](https://github.com/ratatui/ratatui) | Terminal UI framework |
| [crossterm](https://github.com/crossterm-rs/crossterm) | Terminal I/O, keyboard & mouse events |
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite driver (bundled) |
| [duckdb](https://github.com/duckdb/duckdb-rs) | DuckDB driver (bundled) |
| [clap](https://github.com/clap-rs/clap) | CLI argument parsing |
| [dirs](https://github.com/dirs-dev/dirs-rs) | Platform data directory paths |

## Development

```sh
make build       # Debug build
make release     # Release build with LTO
make clippy      # Run clippy lints
make fmt         # Format code
make check       # Type check without building
```
# About

Queries is an experimental SQL client for PostgreSQL and Sqlite3 with support for real-time graphical data visualization.

# Installation

## System requirements

The GNU Scientific Library (![libgsl](https://www.gnu.org/software/gsl/)) should be installed on the system:

```
sudo apt install libgsl23
```

## Build

Queries is available for Linux-based operating systems, and must be compiled from source using ![Cargo](https://www.rust-lang.org/tools/install). 

```shell
git clone https://github.com/limads/queries.git
cd queries
cargo build --release
./target/release/queries
```

# Usage

## Connection 

Queries is not yet ready for remote connections (TLS), although that is a short-term goal. Start a SQL connection to a local PostgreSQL database by clicking in the `Connection` button on the window header bar or pressing `CTRL+C`:

A typical connection to PostgreSQL requires the information:

```
Host: localhost|127.0.0.1
User: [user]
Password: [password]
Database: [dbname]
```

To connect to an existing SQLite database, use the `Open` button on the connection popover to point to a SQLite3 file (`.db|.sqlite` extension). 

Alternatively, just switch the connection button to start a new empty in-memory Sqlite database, and populate it with a sequence of `create table` and `insert` statements.

You can also start an in-memory database by uploading a sequence of CSV-formatted files with the `Open` button. Queries will attempt to translate the delimited values to a sequence of SQL commands to populate the in-memory database. CSV files can be opened with Queries for covenience, but recall that CSV and relational records are representing different things: CSV is an ordered sequence of records, while the result of a SQL query is an (in principle) unordered sequence of records. If you want to preserve the CSV sequence structure, you must have an index column; and invoke the respective `sort by` SQL clause to recover its structure.

## Executing queries

Start a query sequence by opening a `.sql` file at the upper portion of the left sidebar, or start writing a query sequence there (`CTRL+Q`). Click the `Refresh` button or press `CTRL+Enter` to execute the query sequence. Each `select` statement maps to a new table in the main pane of the application. You can also send `insert|update|delete` statements or database administration statements. 

By toggling the `Update` button, you can repeat a `select` statement execution every n seconds, re-populating the table environment and any graphics with the most recent database information.

## Visualization

Queries rely on the (also in early-stage development) ![gtkplotview](https://github.com/limads/gtkplotview) sister project for visualization. First, load a XML `gtkplotview` layout, or start a new one. After a layout is loaded, select 1, 2 or 3 columns from any table in the environment by clicking in their headers, and click the `Add Mapping` button in the lower-left sidebar, or press `CTRL+M`. Select one from the available mappings, and edit its visual properties in the lower-left menu.

Plots can be saved to SVG via the `Export Figure` button on the upper right header menu. To reproduce the visualization at another Queries session, you can also use the export text button, using `.xml` as the extension. This layout can be used as at a new session. If the same table environment is found when the layout is uploaded, Queries will try to map any columns satisfying the same names and positions found at the last session to the current plot. If a column is not found, it is disabilitated until the user selects a new column.

# Development status

1. [X] 1-column visualization (Bars/histograms)

2. [X] 2-column visualization (Lines/Scatters/Textual labels)

3. [ ] 3-column visualization (Area/Surface plots)

4. [X] Multi-plot view (2 or 4 simultaneous plots)

5. [ ] TLS-enabled remote connections

6. [ ] Menu to manage SQLite3 extensions, integrated with Rust's package manager.

7. [ ] Full integration with Dark/White Gnome Shell themes.

# Relevant projects

Queries builds on several recent open-source projects, most notably the Rust/Gtk integration ecosystem ![gtk-rs](https://gtk-rs.org/). Database connectivity to SQLite/Postgres is supported by the crates ![rusqlite](https://crates.io/crates/rusqlite) and ![postgres](https://crates.io/crates/postgres), respectively. Visualization is based on Cairo, via the sister ![gtkplotview](https://github.com/limads/gtkplotview) Rust project.

# License

Queries is licensed under the [GPL v3.0](https://www.gnu.org/licenses/gpl-3.0.en.html).




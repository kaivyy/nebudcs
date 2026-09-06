//! SQLite WAL physical persistence engine, schema migrations, and batch transactions.

use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use std::path::Path;

use crate::buffer::HistorianRecord;
use crate::error::HistorianResult;
use crate::point::HistorianPointConfig;
use crate::run::HistorianRunMeta;
use crate::sample::HistoricalSampleValue;

/// Physical SQLite connection manager and statement cache.
pub struct SqliteStorage {
    conn: Connection,
}

impl SqliteStorage {
    /// Opens or creates an embedded SQLite database at the specified filesystem path.
    ///
    /// Applies WAL mode, cache sizing, and schema tables.
    pub fn open<P: AsRef<Path>>(path: P) -> HistorianResult<Self> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let conn = Connection::open(path_ref)?;
        let mut storage = Self { conn };
        storage.initialize_database()?;
        Ok(storage)
    }

    /// Creates an in-memory SQLite database (for high-speed unit testing).
    pub fn in_memory() -> HistorianResult<Self> {
        let conn = Connection::open_in_memory()?;
        let mut storage = Self { conn };
        storage.initialize_database()?;
        Ok(storage)
    }

    /// Opens a read-only connection to an existing SQLite database.
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> HistorianResult<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        Ok(Self { conn })
    }

    /// Initializes PRAGMAs and creates tables and indices.
    fn initialize_database(&mut self) -> HistorianResult<()> {
        self.conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA busy_timeout = 5000;
             PRAGMA temp_store = MEMORY;
             PRAGMA foreign_keys = ON;",
        )?;

        self.conn.execute_batch(
            "-- Runs Table
            CREATE TABLE IF NOT EXISTS historian_runs (
                run_id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                model_version TEXT NOT NULL,
                started_wall_clock INTEGER NOT NULL,
                ended_wall_clock INTEGER,
                simulation_dt REAL NOT NULL,
                time_scale REAL NOT NULL DEFAULT 1.0,
                comments TEXT
            );

            -- Points Table
            CREATE TABLE IF NOT EXISTS historian_points (
                run_id TEXT NOT NULL,
                point_id INTEGER NOT NULL,
                tag_id TEXT NOT NULL,
                name TEXT NOT NULL,
                quantity_kind TEXT NOT NULL,
                engineering_unit TEXT NOT NULL,
                eng_min REAL NOT NULL,
                eng_max REAL NOT NULL,
                sampling_mode TEXT NOT NULL,
                PRIMARY KEY (run_id, point_id),
                FOREIGN KEY (run_id) REFERENCES historian_runs(run_id) ON DELETE CASCADE
            );

            -- Analog Timeseries Table (Clustered WITHOUT ROWID)
            CREATE TABLE IF NOT EXISTS history_analog (
                run_id TEXT NOT NULL,
                point_id INTEGER NOT NULL,
                sim_time REAL NOT NULL,
                seq INTEGER NOT NULL,
                value REAL NOT NULL,
                quality INTEGER NOT NULL,
                persisted_wall_clock INTEGER NOT NULL,
                PRIMARY KEY (run_id, point_id, sim_time, seq)
            ) WITHOUT ROWID;

            -- Discrete Timeseries Table (Clustered WITHOUT ROWID)
            CREATE TABLE IF NOT EXISTS history_discrete (
                run_id TEXT NOT NULL,
                point_id INTEGER NOT NULL,
                sim_time REAL NOT NULL,
                seq INTEGER NOT NULL,
                value INTEGER NOT NULL,
                quality INTEGER NOT NULL,
                persisted_wall_clock INTEGER NOT NULL,
                PRIMARY KEY (run_id, point_id, sim_time, seq)
            ) WITHOUT ROWID;

            -- Discrete DCS Events Table
            CREATE TABLE IF NOT EXISTS history_events (
                run_id TEXT NOT NULL,
                sim_time REAL NOT NULL,
                seq INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                source_id TEXT NOT NULL,
                old_state TEXT,
                new_state TEXT NOT NULL,
                value REAL,
                severity INTEGER,
                details_json TEXT,
                persisted_wall_clock INTEGER NOT NULL,
                PRIMARY KEY (run_id, sim_time, seq)
            );

            -- Temporal range search indices
            CREATE INDEX IF NOT EXISTS idx_analog_lookup 
            ON history_analog (run_id, point_id, sim_time);

            CREATE INDEX IF NOT EXISTS idx_discrete_lookup 
            ON history_discrete (run_id, point_id, sim_time);

            CREATE INDEX IF NOT EXISTS idx_events_lookup 
            ON history_events (run_id, sim_time);",
        )?;

        Ok(())
    }

    /// Access inner SQLite connection reference.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Access mutable inner connection reference.
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Consumes the storage handle and returns the underlying SQLite connection.
    pub fn into_connection(self) -> Connection {
        self.conn
    }

    /// Registers a new simulation run in the `historian_runs` table.
    pub fn insert_run(&mut self, meta: &HistorianRunMeta) -> HistorianResult<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT INTO historian_runs (
                run_id, project_id, model_version, started_wall_clock,
                ended_wall_clock, simulation_dt, time_scale, comments
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
        )?;

        stmt.execute(params![
            meta.run_id,
            meta.project_id.to_string(),
            meta.model_version,
            meta.started_wall_clock,
            meta.ended_wall_clock,
            meta.simulation_dt,
            meta.time_scale,
            meta.comments,
        ])?;

        Ok(())
    }

    /// Registers a set of point configurations for a specific run.
    pub fn insert_points(
        &mut self,
        run_id: &str,
        points: &[HistorianPointConfig],
    ) -> HistorianResult<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR REPLACE INTO historian_points (
                    run_id, point_id, tag_id, name, quantity_kind,
                    engineering_unit, eng_min, eng_max, sampling_mode
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            )?;

            for pt in points {
                let mode_json = serde_json::to_string(&pt.sampling_mode)?;
                let kind_json = serde_json::to_string(&pt.quantity_kind)?;
                stmt.execute(params![
                    run_id,
                    pt.point_id.as_u32(),
                    pt.tag_id.to_string(),
                    pt.name,
                    kind_json,
                    pt.engineering_unit,
                    pt.eng_min,
                    pt.eng_max,
                    mode_json,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Writes a batch of historical records within an atomic transaction.
    ///
    /// Evaluates prepared statements for analog samples, discrete samples, and events.
    pub fn write_batch(
        &mut self,
        active_run_id: &str,
        records: &[HistorianRecord],
    ) -> HistorianResult<usize> {
        if records.is_empty() {
            return Ok(0);
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut written = 0;
        {
            let mut stmt_analog = tx.prepare_cached(
                "INSERT OR REPLACE INTO history_analog (
                    run_id, point_id, sim_time, seq, value, quality, persisted_wall_clock
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            )?;

            let mut stmt_discrete = tx.prepare_cached(
                "INSERT OR REPLACE INTO history_discrete (
                    run_id, point_id, sim_time, seq, value, quality, persisted_wall_clock
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            )?;

            let mut stmt_event = tx.prepare_cached(
                "INSERT OR REPLACE INTO history_events (
                    run_id, sim_time, seq, event_type, source_id, old_state,
                    new_state, value, severity, details_json, persisted_wall_clock
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11);",
            )?;

            for rec in records {
                match rec {
                    HistorianRecord::Sample(sample) => {
                        let quality_code = match sample.quality {
                            nebudcs_model::tag::TagQuality::Good => 0,
                            nebudcs_model::tag::TagQuality::Uncertain => 1,
                            nebudcs_model::tag::TagQuality::Bad => 2,
                            nebudcs_model::tag::TagQuality::NotConnected => 3,
                        };

                        match sample.value {
                            HistoricalSampleValue::Float(v) => {
                                stmt_analog.execute(params![
                                    active_run_id,
                                    sample.point_id.as_u32(),
                                    sample.sim_time.seconds(),
                                    sample.seq,
                                    v,
                                    quality_code,
                                    sample.persisted_wall_clock_ms,
                                ])?;
                            }
                            HistoricalSampleValue::Discrete(i) => {
                                stmt_discrete.execute(params![
                                    active_run_id,
                                    sample.point_id.as_u32(),
                                    sample.sim_time.seconds(),
                                    sample.seq,
                                    i,
                                    quality_code,
                                    sample.persisted_wall_clock_ms,
                                ])?;
                            }
                        }
                        written += 1;
                    }
                    HistorianRecord::Event(ev) => {
                        stmt_event.execute(params![
                            if ev.run_id.is_empty() {
                                active_run_id
                            } else {
                                &ev.run_id
                            },
                            ev.sim_time.seconds(),
                            ev.seq,
                            ev.event_type,
                            ev.source_id,
                            ev.old_state,
                            ev.new_state,
                            ev.value,
                            ev.severity,
                            ev.details_json,
                            ev.persisted_wall_clock_ms,
                        ])?;
                        written += 1;
                    }
                    HistorianRecord::DataLossGap {
                        sim_time,
                        seq,
                        dropped_count,
                    } => {
                        let details = format!("{{\"dropped_samples_count\":{}}}", dropped_count);
                        stmt_event.execute(params![
                            active_run_id,
                            sim_time.seconds(),
                            seq,
                            "DATA_LOSS_GAP",
                            "HISTORIAN_QUEUE",
                            Option::<String>::None,
                            "DROPPED",
                            Option::<f64>::None,
                            3, // Warning severity
                            Some(details),
                            0i64,
                        ])?;
                        written += 1;
                    }
                    HistorianRecord::FlushSync => {}
                }
            }
        }

        tx.commit()?;
        Ok(written)
    }

    /// Performs an integrity check on the SQLite database.
    pub fn check_integrity(&self) -> HistorianResult<bool> {
        let mut stmt = self.conn.prepare("PRAGMA quick_check;")?;
        let result: String = stmt.query_row([], |row| row.get(0))?;
        Ok(result == "ok")
    }
}

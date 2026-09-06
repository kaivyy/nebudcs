//! Storage-independent historian reader trait and query request structures.

use nebudcs_model::tag::TagQuality;
use nebudcs_types::SimulationTime;
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;

use crate::aggregate::{downsample_to_buckets, AggregatedBucket};
use crate::error::{HistorianError, HistorianResult};
use crate::event::HistoricalEvent;
use crate::point::{HistorianPointConfig, HistorianPointId, SamplingMode};
use crate::sample::RawSample;

/// Request to fetch raw contiguous historical samples.
#[derive(Debug, Clone)]
pub struct RawQueryRequest {
    pub run_id: String,
    pub point_id: HistorianPointId,
    pub start_time: SimulationTime,
    pub end_time: SimulationTime,
    pub limit: Option<usize>,
}

/// Request to fetch downsampled, bucket-aggregated timeseries.
#[derive(Debug, Clone)]
pub struct AggregatedQueryRequest {
    pub run_id: String,
    pub point_id: HistorianPointId,
    pub start_time: SimulationTime,
    pub end_time: SimulationTime,
    pub bucket_count: usize,
}

/// Storage-independent trait for reading historical simulation data.
pub trait HistorianReader: Send + Sync {
    /// Lists all points registered under a specific simulation run.
    fn list_points(&self, run_id: &str) -> HistorianResult<Vec<HistorianPointConfig>>;

    /// Queries raw individual samples over a given simulation time slice.
    fn query_raw(&self, req: RawQueryRequest) -> HistorianResult<Vec<RawSample>>;

    /// Queries bucket-aggregated downsampled points preserving min/max excursions.
    fn query_aggregated(
        &self,
        req: AggregatedQueryRequest,
    ) -> HistorianResult<Vec<AggregatedBucket>>;

    /// Queries discrete DCS operational and alarm events over a time slice.
    fn query_events(
        &self,
        run_id: &str,
        start: SimulationTime,
        end: SimulationTime,
    ) -> HistorianResult<Vec<HistoricalEvent>>;
}

/// Production SQLite implementation of [`HistorianReader`].
pub struct SqliteHistorianReader {
    conn: Mutex<Connection>,
}

impl SqliteHistorianReader {
    /// Opens a read-only reader connection to an existing SQLite database.
    pub fn open<P: AsRef<Path>>(path: P) -> HistorianResult<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        // Ensure WAL mode read optimization
        conn.execute_batch("PRAGMA busy_timeout = 5000; PRAGMA query_only = ON;")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Creates a reader from an in-memory or already opened connection.
    pub fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

impl HistorianReader for SqliteHistorianReader {
    fn list_points(&self, run_id: &str) -> HistorianResult<Vec<HistorianPointConfig>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT point_id, tag_id, name, quantity_kind, engineering_unit,
                    eng_min, eng_max, sampling_mode
             FROM historian_points
             WHERE run_id = ?1
             ORDER BY point_id ASC;",
        )?;

        let rows = stmt.query_map(params![run_id], |row| {
            let point_id: u32 = row.get(0)?;
            let tag_id_str: String = row.get(1)?;
            let name: String = row.get(2)?;
            let kind_str: String = row.get(3)?;
            let unit: String = row.get(4)?;
            let eng_min: f64 = row.get(5)?;
            let eng_max: f64 = row.get(6)?;
            let mode_json: String = row.get(7)?;

            Ok((
                point_id, tag_id_str, name, kind_str, unit, eng_min, eng_max, mode_json,
            ))
        })?;

        let mut points = Vec::new();
        for r in rows {
            let (pid, tid_str, name, kind_str, unit, min, max, mode_json) = r?;
            let tag_id: nebudcs_types::TagId = tid_str
                .parse()
                .map_err(|e| HistorianError::SecurityError(format!("Invalid tag ID in DB: {e}")))?;
            let mode: SamplingMode = serde_json::from_str(&mode_json)?;
            let quantity_kind: Option<nebudcs_units::QuantityKind> =
                serde_json::from_str(&kind_str).unwrap_or(None);

            points.push(HistorianPointConfig {
                point_id: HistorianPointId::new(pid),
                tag_id,
                name,
                quantity_kind,
                engineering_unit: unit,
                eng_min: min,
                eng_max: max,
                sampling_mode: mode,
            });
        }

        Ok(points)
    }

    fn query_raw(&self, req: RawQueryRequest) -> HistorianResult<Vec<RawSample>> {
        if req.start_time.seconds() > req.end_time.seconds() {
            return Err(HistorianError::InvalidTimeRange {
                start: req.start_time.seconds(),
                end: req.end_time.seconds(),
            });
        }

        let conn = self.conn.lock().unwrap();

        // 1. Check analog table
        let mut sql = "SELECT sim_time, seq, value, quality, persisted_wall_clock
                       FROM history_analog
                       WHERE run_id = ?1 AND point_id = ?2 AND sim_time >= ?3 AND sim_time <= ?4
                       ORDER BY sim_time ASC, seq ASC"
            .to_string();

        if let Some(limit) = req.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![
                req.run_id,
                req.point_id.as_u32(),
                req.start_time.seconds(),
                req.end_time.seconds(),
            ],
            |row| {
                let sim_time_f: f64 = row.get(0)?;
                let seq: u64 = row.get(1)?;
                let val: f64 = row.get(2)?;
                let qual_code: u8 = row.get(3)?;
                let wall_clock: i64 = row.get(4)?;

                let quality = match qual_code {
                    0 => TagQuality::Good,
                    1 => TagQuality::Uncertain,
                    2 => TagQuality::Bad,
                    _ => TagQuality::NotConnected,
                };

                Ok(RawSample {
                    point_id: req.point_id,
                    sim_time: SimulationTime::try_new(sim_time_f).unwrap_or(SimulationTime::ZERO),
                    seq,
                    value: val,
                    quality,
                    persisted_wall_clock_ms: wall_clock,
                })
            },
        )?;

        let mut samples = Vec::new();
        for s in rows {
            samples.push(s?);
        }

        // 2. If analog table was empty, check discrete table
        if samples.is_empty() {
            let mut sql_discrete = "SELECT sim_time, seq, value, quality, persisted_wall_clock
                                    FROM history_discrete
                                    WHERE run_id = ?1 AND point_id = ?2 AND sim_time >= ?3 AND sim_time <= ?4
                                    ORDER BY sim_time ASC, seq ASC".to_string();
            if let Some(limit) = req.limit {
                sql_discrete.push_str(&format!(" LIMIT {}", limit));
            }

            let mut stmt_disc = conn.prepare(&sql_discrete)?;
            let rows_disc = stmt_disc.query_map(
                params![
                    req.run_id,
                    req.point_id.as_u32(),
                    req.start_time.seconds(),
                    req.end_time.seconds(),
                ],
                |row| {
                    let sim_time_f: f64 = row.get(0)?;
                    let seq: u64 = row.get(1)?;
                    let val_i: i64 = row.get(2)?;
                    let qual_code: u8 = row.get(3)?;
                    let wall_clock: i64 = row.get(4)?;

                    let quality = match qual_code {
                        0 => TagQuality::Good,
                        1 => TagQuality::Uncertain,
                        2 => TagQuality::Bad,
                        _ => TagQuality::NotConnected,
                    };

                    Ok(RawSample {
                        point_id: req.point_id,
                        sim_time: SimulationTime::try_new(sim_time_f)
                            .unwrap_or(SimulationTime::ZERO),
                        seq,
                        value: val_i as f64,
                        quality,
                        persisted_wall_clock_ms: wall_clock,
                    })
                },
            )?;

            for s in rows_disc {
                samples.push(s?);
            }
        }

        Ok(samples)
    }

    fn query_aggregated(
        &self,
        req: AggregatedQueryRequest,
    ) -> HistorianResult<Vec<AggregatedBucket>> {
        let raw_samples = self.query_raw(RawQueryRequest {
            run_id: req.run_id,
            point_id: req.point_id,
            start_time: req.start_time,
            end_time: req.end_time,
            limit: None,
        })?;

        let buckets = downsample_to_buckets(
            &raw_samples,
            req.start_time.seconds(),
            req.end_time.seconds(),
            req.bucket_count,
        );

        Ok(buckets)
    }

    fn query_events(
        &self,
        run_id: &str,
        start: SimulationTime,
        end: SimulationTime,
    ) -> HistorianResult<Vec<HistoricalEvent>> {
        if start.seconds() > end.seconds() {
            return Err(HistorianError::InvalidTimeRange {
                start: start.seconds(),
                end: end.seconds(),
            });
        }

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT sim_time, seq, event_type, source_id, old_state,
                    new_state, value, severity, details_json, persisted_wall_clock
             FROM history_events
             WHERE run_id = ?1 AND sim_time >= ?2 AND sim_time <= ?3
             ORDER BY sim_time ASC, seq ASC;",
        )?;

        let rows = stmt.query_map(params![run_id, start.seconds(), end.seconds()], |row| {
            let sim_time_f: f64 = row.get(0)?;
            let seq: u64 = row.get(1)?;
            let event_type: String = row.get(2)?;
            let source_id: String = row.get(3)?;
            let old_state: Option<String> = row.get(4)?;
            let new_state: String = row.get(5)?;
            let value: Option<f64> = row.get(6)?;
            let severity: Option<u8> = row.get(7)?;
            let details_json: Option<String> = row.get(8)?;
            let wall_clock: i64 = row.get(9)?;

            Ok(HistoricalEvent {
                run_id: run_id.to_string(),
                sim_time: SimulationTime::try_new(sim_time_f).unwrap_or(SimulationTime::ZERO),
                seq,
                event_type,
                source_id,
                old_state,
                new_state,
                value,
                severity,
                details_json,
                persisted_wall_clock_ms: wall_clock,
            })
        })?;

        let mut events = Vec::new();
        for e in rows {
            events.push(e?);
        }

        Ok(events)
    }
}

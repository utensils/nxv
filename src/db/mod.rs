//! Database module for nxv index storage.

pub mod import;
pub mod queries;
pub mod releases;

use crate::error::{NxvError, Result};
use queries::PackageVersion;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::time::Duration;

/// Default timeout for SQLite busy handler (in seconds).
/// When the database is locked, SQLite will retry for this duration before returning SQLITE_BUSY.
const DEFAULT_BUSY_TIMEOUT_SECS: u64 = 5;

/// Current schema version.
///
/// v4 (the snapshot indexer): one row per `(attribute_path, version)` with
/// widen-only observation bounds, plus the `releases` ingestion ledger.
pub const SCHEMA_VERSION: u32 = 4;

/// Minimum schema version this build can read.
/// Indexes with min_schema_version > this value are incompatible.
pub const MIN_READABLE_SCHEMA: u32 = 4;

/// Database connection wrapper.
pub struct Database {
    conn: Connection,
}

/// Statistics from a `dedupe_ranges` run.
#[cfg_attr(not(feature = "indexer"), allow(dead_code))]
#[derive(Debug, Clone, Copy, Default)]
pub struct DedupeStats {
    /// Distinct `(attribute_path, version)` pairs found.
    pub groups_total: u64,
    /// Pairs that had more than one row.
    pub groups_with_duplicates: u64,
    /// Total row count before dedupe.
    pub rows_before: u64,
    /// Total row count after dedupe.
    pub rows_after: u64,
    /// Survivor rows that were updated with coalesced range metadata.
    pub rows_updated: u64,
    /// Duplicate rows that were deleted.
    pub rows_deleted: u64,
}

impl Database {
    /// Open or create a database at the given path.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent performance and durability
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA wal_autocheckpoint = 1000;
            "#,
        )?;

        let mut db = Self { conn };
        db.init_schema()?;
        db.migrate_if_needed()?;
        Ok(db)
    }

    /// Checkpoint the WAL to ensure data is flushed to disk.
    /// Call this at regular intervals during long-running operations.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn checkpoint(&self) -> Result<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    /// Open a database in read-only mode.
    ///
    /// Validates that the database schema is compatible with this version of nxv.
    /// Returns an error if the database was created with a newer, incompatible schema version.
    ///
    /// The connection is configured with a busy timeout to prevent indefinite blocking
    /// when the database is locked by another process.
    pub fn open_readonly<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(NxvError::NoIndex);
        }
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        // Set busy timeout to prevent indefinite blocking on database locks.
        // This is critical for preventing thread pool exhaustion under load.
        conn.busy_timeout(Duration::from_secs(DEFAULT_BUSY_TIMEOUT_SECS))?;

        let db = Self { conn };

        // Validate schema version compatibility
        db.validate_schema_version()?;

        Ok(db)
    }

    /// Validate that the database schema is compatible with this version of nxv.
    fn validate_schema_version(&self) -> Result<()> {
        // Check if meta table exists (very old or corrupt database)
        let has_meta: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='meta'",
            [],
            |row| row.get(0),
        )?;

        if !has_meta {
            return Err(NxvError::CorruptIndex("missing meta table".to_string()));
        }

        // Check min_schema_version if set by the indexer (for future schema changes).
        // Falls back to schema_version for indexes that don't have min_schema_version yet.
        let min_version_str = match self.get_meta("min_schema_version")? {
            Some(v) => Some(v),
            None => self.get_meta("schema_version")?,
        };
        let min_version_str = min_version_str.as_deref().unwrap_or("0");
        let min_schema_version: u32 = min_version_str.parse().map_err(|_| {
            NxvError::CorruptIndex(format!(
                "invalid min_schema_version '{}': expected integer",
                min_version_str
            ))
        })?;

        if min_schema_version > MIN_READABLE_SCHEMA {
            return Err(NxvError::IncompatibleIndex(format!(
                "index requires schema version {} but this build only supports up to {}. \
                 Please upgrade nxv to use this index.",
                min_schema_version, MIN_READABLE_SCHEMA
            )));
        }

        Ok(())
    }

    /// Initializes the database schema and related search index.
    ///
    /// Creates the `meta` and `package_versions` tables (including the `source_path` column),
    /// common indexes, and a persistent FTS5 virtual table `package_versions_fts` with triggers
    /// to keep it synchronized with `package_versions`. If the `schema_version` metadata entry
    /// is missing, sets it to the current SCHEMA_VERSION.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the schema is present or was created successfully, `Err(_)` if a database
    /// operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::db::Database;
    /// let db = Database::open(":memory:").unwrap();
    /// db.init_schema().unwrap();
    /// ```
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            -- Track indexing state and metadata
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Main package version table. One row per (attribute_path, version):
            -- "this pair was OBSERVED at first_commit and at last_commit". Interior
            -- presence is interpolated, not guaranteed (see DESIGN.md range semantics).
            CREATE TABLE IF NOT EXISTS package_versions (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                first_commit_hash TEXT NOT NULL,
                first_commit_date INTEGER NOT NULL,
                last_commit_hash TEXT NOT NULL,
                last_commit_date INTEGER NOT NULL,
                attribute_path TEXT NOT NULL,
                description TEXT,
                license TEXT,
                homepage TEXT,
                maintainers TEXT,
                platforms TEXT,
                source_path TEXT,
                known_vulnerabilities TEXT,
                UNIQUE(attribute_path, version),
                CHECK(first_commit_date <= last_commit_date)
            );

            -- Channel-release ingestion ledger: which snapshots have been observed,
            -- which are pending/failed, and their retry/backoff state.
            CREATE TABLE IF NOT EXISTS releases (
                id INTEGER PRIMARY KEY,
                channel TEXT NOT NULL,
                release_name TEXT NOT NULL,
                commit_hash TEXT NOT NULL,
                commit_count INTEGER,
                release_date INTEGER NOT NULL,
                source TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                attempts INTEGER NOT NULL DEFAULT 0,
                last_attempt_at INTEGER,
                attr_count INTEGER,
                error TEXT,
                ingested_at INTEGER,
                UNIQUE(channel, release_name)
            );
            CREATE INDEX IF NOT EXISTS idx_releases_status ON releases(status, release_date);
            CREATE INDEX IF NOT EXISTS idx_releases_channel_date ON releases(channel, release_date DESC);

            -- Indexes for common query patterns
            CREATE INDEX IF NOT EXISTS idx_packages_name ON package_versions(name);
            CREATE INDEX IF NOT EXISTS idx_packages_name_version ON package_versions(name, version, first_commit_date);
            CREATE INDEX IF NOT EXISTS idx_packages_attr ON package_versions(attribute_path);
            CREATE INDEX IF NOT EXISTS idx_packages_first_date ON package_versions(first_commit_date DESC);
            CREATE INDEX IF NOT EXISTS idx_packages_last_date ON package_versions(last_commit_date DESC);
            "#,
        )?;

        // Create FTS5 table if it doesn't exist
        let fts_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='package_versions_fts'",
            [],
            |row| row.get(0),
        )?;

        if !fts_exists {
            self.conn.execute_batch(
                r#"
                CREATE VIRTUAL TABLE package_versions_fts
                USING fts5(name, description, content=package_versions, content_rowid=id);

                -- Triggers to keep FTS5 in sync with package_versions
                CREATE TRIGGER IF NOT EXISTS package_versions_ai AFTER INSERT ON package_versions BEGIN
                    INSERT INTO package_versions_fts(rowid, name, description)
                    VALUES (new.id, new.name, new.description);
                END;

                CREATE TRIGGER IF NOT EXISTS package_versions_ad AFTER DELETE ON package_versions BEGIN
                    INSERT INTO package_versions_fts(package_versions_fts, rowid, name, description)
                    VALUES ('delete', old.id, old.name, old.description);
                END;

                -- Guarded: pure range-widening updates (the indexer's hot path —
                -- every alive attr per snapshot) must not churn the FTS index.
                CREATE TRIGGER IF NOT EXISTS package_versions_au AFTER UPDATE ON package_versions
                WHEN old.name IS NOT new.name OR old.description IS NOT new.description
                BEGIN
                    INSERT INTO package_versions_fts(package_versions_fts, rowid, name, description)
                    VALUES ('delete', old.id, old.name, old.description);
                    INSERT INTO package_versions_fts(rowid, name, description)
                    VALUES (new.id, new.name, new.description);
                END;
                "#,
            )?;
        }

        // Create vulnerability index if column exists (for new databases)
        // This index is conditional because old databases being migrated won't have the column yet
        let has_known_vulns: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM pragma_table_info('package_versions') WHERE name='known_vulnerabilities'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if has_known_vulns {
            self.conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_version_vulnerabilities ON package_versions(version) WHERE known_vulnerabilities IS NOT NULL AND known_vulnerabilities != '' AND known_vulnerabilities != '[]' AND known_vulnerabilities != 'null'",
                [],
            )?;
        }

        // Set schema version if not already set
        let version = self.get_meta("schema_version")?;
        if version.is_none() {
            self.set_meta("schema_version", &SCHEMA_VERSION.to_string())?;
        }

        Ok(())
    }

    /// Apply any pending schema migrations to the database.
    ///
    /// This updates the on-disk schema to the module's current `SCHEMA_VERSION`.
    /// Specifically, when upgrading from versions earlier than 2 it adds the
    /// `source_path` TEXT column to the `package_versions` table if that column is
    /// not already present, and then writes the new `schema_version` into the
    /// `meta` table.
    ///
    /// # Examples
    ///
    /// ```
    /// let db = Database::open(std::path::Path::new(":memory:")).unwrap();
    /// db.migrate_if_needed().unwrap();
    /// ```
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    fn migrate_if_needed(&mut self) -> Result<()> {
        let version_str = self.get_meta("schema_version")?;
        let current_version: u32 = version_str.as_deref().unwrap_or("0").parse().unwrap_or(0);

        if current_version < 2 {
            // Migration v1 -> v2: Add source_path column
            let has_source_path: bool = self
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info('package_versions') WHERE name='source_path'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !has_source_path {
                self.conn.execute(
                    "ALTER TABLE package_versions ADD COLUMN source_path TEXT",
                    [],
                )?;
            }
        }

        if current_version < 3 {
            // Migration v2 -> v3: Add known_vulnerabilities column
            let has_known_vulns: bool = self
                .conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info('package_versions') WHERE name='known_vulnerabilities'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !has_known_vulns {
                self.conn.execute(
                    "ALTER TABLE package_versions ADD COLUMN known_vulnerabilities TEXT",
                    [],
                )?;
            }

            // Add index for efficient vulnerability lookups in version history queries
            self.conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_version_vulnerabilities ON package_versions(version) WHERE known_vulnerabilities IS NOT NULL AND known_vulnerabilities != '' AND known_vulnerabilities != '[]' AND known_vulnerabilities != 'null'",
                [],
            )?;
        }

        if current_version > 0 && current_version < 4 {
            // Migration v3 -> v4: tighten the unique key to (attribute_path, version).
            //
            // The v4 widen-only upsert targets ON CONFLICT(attribute_path, version),
            // which needs a matching unique index. Pre-v4 DBs (a) may hold duplicate
            // (attr, version) rows from the old incremental indexer, and (b) only
            // have the legacy 3-column unique constraint — which stays behind as a
            // harmlessly redundant index (ALTER TABLE can't drop it without a table
            // rebuild, and the new index subsumes it).
            self.dedupe_ranges(false)?;
            self.conn.execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS uq_attr_version
                 ON package_versions(attribute_path, version)",
                [],
            )?;

            // The old AFTER UPDATE FTS trigger fires on every range-widening
            // update; replace it with the guarded variant (matches init_schema).
            self.conn.execute_batch(
                r#"
                DROP TRIGGER IF EXISTS package_versions_au;
                CREATE TRIGGER package_versions_au AFTER UPDATE ON package_versions
                WHEN old.name IS NOT new.name OR old.description IS NOT new.description
                BEGIN
                    INSERT INTO package_versions_fts(package_versions_fts, rowid, name, description)
                    VALUES ('delete', old.id, old.name, old.description);
                    INSERT INTO package_versions_fts(rowid, name, description)
                    VALUES (new.id, new.name, new.description);
                END;
                "#,
            )?;

            // Retired with the checkpoint-resume model.
            self.conn
                .execute("DROP INDEX IF EXISTS idx_packages_last_commit_hash", [])?;
        }

        if current_version < SCHEMA_VERSION {
            self.set_meta("schema_version", &SCHEMA_VERSION.to_string())?;
        }

        Ok(())
    }

    /// Get a metadata value by key.
    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let result = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?", [key], |row| {
                row.get(0)
            });

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Set a metadata value.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    /// Get the underlying connection for advanced operations.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Upserts package version *observations* in a single transaction.
    ///
    /// Keyed on `UNIQUE(attribute_path, version)`. Bounds only ever widen
    /// (order-agnostic: parallel and out-of-order ingestion are safe), and
    /// metadata follows the **newest** observation — an older snapshot
    /// arriving late can extend `first_*` backwards but never overwrite
    /// metadata written by a newer one. `source_path` is additionally sticky
    /// against NULL (a newer snapshot without position info keeps the old one).
    ///
    /// SQLite evaluates all `DO UPDATE SET` right-hand sides against the
    /// pre-update row, so the `last_commit_date` comparisons below are
    /// consistent regardless of column order.
    ///
    /// # Returns
    ///
    /// The number of row operations (inserts and updates combined).
    // Public write API; the indexer writes through commit_flush_group, tests
    // and external tooling use this directly.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn upsert_observations(&mut self, packages: &[PackageVersion]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let written = Self::upsert_observations_tx(&tx, packages)?;
        tx.commit()?;
        Ok(written)
    }

    /// Like [`Self::upsert_observations`] but runs inside a caller-provided
    /// transaction, so row writes can commit atomically with `releases`
    /// status updates.
    #[allow(dead_code)]
    pub(crate) fn upsert_observations_tx(
        tx: &rusqlite::Transaction<'_>,
        packages: &[PackageVersion],
    ) -> Result<usize> {
        let mut written = 0;
        let mut stmt = tx.prepare_cached(
            r#"
            INSERT INTO package_versions
                (name, version, first_commit_hash, first_commit_date,
                 last_commit_hash, last_commit_date, attribute_path,
                 description, license, homepage, maintainers, platforms, source_path,
                 known_vulnerabilities)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(attribute_path, version) DO UPDATE SET
                first_commit_hash = CASE
                    WHEN excluded.first_commit_date < first_commit_date
                    THEN excluded.first_commit_hash ELSE first_commit_hash END,
                first_commit_date = MIN(first_commit_date, excluded.first_commit_date),
                last_commit_hash = CASE
                    WHEN excluded.last_commit_date > last_commit_date
                    THEN excluded.last_commit_hash ELSE last_commit_hash END,
                name = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.name ELSE name END,
                description = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.description ELSE description END,
                license = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.license ELSE license END,
                homepage = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.homepage ELSE homepage END,
                maintainers = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.maintainers ELSE maintainers END,
                platforms = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.platforms ELSE platforms END,
                source_path = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN COALESCE(excluded.source_path, source_path) ELSE source_path END,
                known_vulnerabilities = CASE
                    WHEN excluded.last_commit_date >= last_commit_date
                    THEN excluded.known_vulnerabilities ELSE known_vulnerabilities END,
                last_commit_date = MAX(last_commit_date, excluded.last_commit_date)
            "#,
        )?;

        for pkg in packages {
            let changes = stmt.execute(rusqlite::params![
                pkg.name,
                pkg.version,
                pkg.first_commit_hash,
                pkg.first_commit_date.timestamp(),
                pkg.last_commit_hash,
                pkg.last_commit_date.timestamp(),
                pkg.attribute_path,
                pkg.description,
                pkg.license,
                pkg.homepage,
                pkg.maintainers,
                pkg.platforms,
                pkg.source_path,
                pkg.known_vulnerabilities,
            ])?;
            written += changes;
        }

        Ok(written)
    }

    /// Drop the FTS sync triggers for bulk ingestion. Pair with
    /// [`Self::rebuild_fts`] (which also recreates the triggers) in a
    /// `--full`/catch-up run's finish step; measured 28x writer speedup.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn drop_fts_triggers(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            DROP TRIGGER IF EXISTS package_versions_ai;
            DROP TRIGGER IF EXISTS package_versions_ad;
            DROP TRIGGER IF EXISTS package_versions_au;
            "#,
        )?;
        Ok(())
    }

    /// True when the FTS table exists but its sync triggers don't — the
    /// state left behind if a bulk run died between [`Self::drop_fts_triggers`]
    /// and [`Self::rebuild_fts`]. Callers should rebuild when this is set.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn fts_triggers_missing(&self) -> Result<bool> {
        let fts_exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='package_versions_fts'",
            [],
            |row| row.get(0),
        )?;
        if !fts_exists {
            return Ok(false);
        }
        let trigger_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name IN \
             ('package_versions_ai', 'package_versions_ad', 'package_versions_au')",
            [],
            |row| row.get(0),
        )?;
        Ok(trigger_count < 3)
    }

    /// Rebuild the FTS index from the content table and recreate the sync
    /// triggers dropped by [`Self::drop_fts_triggers`].
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn rebuild_fts(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            INSERT INTO package_versions_fts(package_versions_fts) VALUES('rebuild');

            CREATE TRIGGER IF NOT EXISTS package_versions_ai AFTER INSERT ON package_versions BEGIN
                INSERT INTO package_versions_fts(rowid, name, description)
                VALUES (new.id, new.name, new.description);
            END;

            CREATE TRIGGER IF NOT EXISTS package_versions_ad AFTER DELETE ON package_versions BEGIN
                INSERT INTO package_versions_fts(package_versions_fts, rowid, name, description)
                VALUES ('delete', old.id, old.name, old.description);
            END;

            CREATE TRIGGER IF NOT EXISTS package_versions_au AFTER UPDATE ON package_versions
            WHEN old.name IS NOT new.name OR old.description IS NOT new.description
            BEGIN
                INSERT INTO package_versions_fts(package_versions_fts, rowid, name, description)
                VALUES ('delete', old.id, old.name, old.description);
                INSERT INTO package_versions_fts(rowid, name, description)
                VALUES (new.id, new.name, new.description);
            END;
            "#,
        )?;
        Ok(())
    }

    /// Collapse duplicate `(attribute_path, version)` rows into one.
    ///
    /// For every pair with more than one row, keep the row with the earliest
    /// `first_commit_date` (ties broken by smallest `id`), extend its
    /// `last_commit_*` fields to the latest values seen across the group, and
    /// delete the losers. Used to repair databases bloated by the pre-0.1.5
    /// incremental-indexer bug.
    ///
    /// Metadata (description, license, homepage, maintainers, platforms,
    /// source_path, known_vulnerabilities) is retained from the surviving row.
    /// A subsequent incremental indexing run will refresh those fields via
    /// upsert as packages appear in new commits.
    ///
    /// Returns statistics about the operation. Does not VACUUM — callers that
    /// want to reclaim disk space should run `VACUUM` separately. If `dry_run`
    /// is true the computation is performed but no rows are modified.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn dedupe_ranges(&mut self, dry_run: bool) -> Result<DedupeStats> {
        let rows_before: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
                    row.get::<_, i64>(0)
                })? as u64;

        struct Plan {
            survivor_id: i64,
            canon_first_hash: String,
            canon_first_date: i64,
            canon_last_id: i64,
            canon_last_hash: String,
            canon_last_date: i64,
            loser_ids: Vec<i64>,
        }

        // Helper index to make the ordered scan below cheap on large DBs.
        // SQLite temp indexes can't cover main-schema tables, so we create
        // an ordinary index scoped to the read phase and use a RAII guard to
        // drop it on any exit path (early return, error, panic). The index
        // is gone before the write transaction starts.
        let mut plans: Vec<Plan> = Vec::new();
        {
            struct IndexGuard<'a> {
                conn: &'a Connection,
            }
            impl Drop for IndexGuard<'_> {
                fn drop(&mut self) {
                    let _ = self
                        .conn
                        .execute_batch("DROP INDEX IF EXISTS temp_idx_dedupe_sort;");
                }
            }

            self.conn.execute_batch(
                "CREATE INDEX IF NOT EXISTS temp_idx_dedupe_sort \
                 ON package_versions (attribute_path, version, first_commit_date, id);",
            )?;
            let _index_guard = IndexGuard { conn: &self.conn };

            let mut stmt = self.conn.prepare(
                "SELECT id, attribute_path, version, \
                        first_commit_hash, first_commit_date, \
                        last_commit_hash, last_commit_date \
                   FROM package_versions \
                  ORDER BY attribute_path, version, first_commit_date ASC, id ASC",
            )?;
            let mut rows = stmt.query([])?;

            let mut current_key: Option<(String, String)> = None;
            let mut current: Option<Plan> = None;

            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let ap: String = row.get(1)?;
                let ver: String = row.get(2)?;
                let fch: String = row.get(3)?;
                let fcd: i64 = row.get(4)?;
                let lch: String = row.get(5)?;
                let lcd: i64 = row.get(6)?;
                let key = (ap, ver);

                if current_key.as_ref() != Some(&key) {
                    if let Some(plan) = current.take() {
                        plans.push(plan);
                    }
                    current = Some(Plan {
                        survivor_id: id,
                        canon_first_hash: fch,
                        canon_first_date: fcd,
                        canon_last_id: id,
                        canon_last_hash: lch,
                        canon_last_date: lcd,
                        loser_ids: Vec::new(),
                    });
                    current_key = Some(key);
                } else {
                    let plan = current.as_mut().unwrap();
                    plan.loser_ids.push(id);
                    // Deterministic tiebreak: strictly greater last_commit_date,
                    // or equal date with a larger row id than the current holder
                    // of canon_last_*.
                    if lcd > plan.canon_last_date
                        || (lcd == plan.canon_last_date && id > plan.canon_last_id)
                    {
                        plan.canon_last_id = id;
                        plan.canon_last_hash = lch;
                        plan.canon_last_date = lcd;
                    }
                }
            }
            if let Some(plan) = current.take() {
                plans.push(plan);
            }
        }

        let groups_total = plans.len() as u64;
        let groups_with_duplicates =
            plans.iter().filter(|p| !p.loser_ids.is_empty()).count() as u64;

        if dry_run {
            let projected_deletes: u64 = plans.iter().map(|p| p.loser_ids.len() as u64).sum();
            return Ok(DedupeStats {
                groups_total,
                groups_with_duplicates,
                rows_before,
                rows_after: rows_before - projected_deletes,
                rows_updated: groups_with_duplicates,
                rows_deleted: projected_deletes,
            });
        }

        let mut rows_updated: u64 = 0;
        let mut rows_deleted: u64 = 0;

        let tx = self.conn.transaction()?;
        {
            let mut update_stmt = tx.prepare_cached(
                "UPDATE package_versions \
                    SET first_commit_hash = ?, \
                        first_commit_date = ?, \
                        last_commit_hash = ?, \
                        last_commit_date = ? \
                  WHERE id = ?",
            )?;
            let mut delete_stmt = tx.prepare_cached("DELETE FROM package_versions WHERE id = ?")?;

            for plan in &plans {
                if plan.loser_ids.is_empty() {
                    continue;
                }
                rows_updated += update_stmt.execute(rusqlite::params![
                    plan.canon_first_hash,
                    plan.canon_first_date,
                    plan.canon_last_hash,
                    plan.canon_last_date,
                    plan.survivor_id,
                ])? as u64;
                for loser_id in &plan.loser_ids {
                    rows_deleted += delete_stmt.execute([loser_id])? as u64;
                }
            }
        }
        tx.commit()?;

        let rows_after: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
                    row.get::<_, i64>(0)
                })? as u64;

        Ok(DedupeStats {
            groups_total,
            groups_with_duplicates,
            rows_before,
            rows_after,
            rows_updated,
            rows_deleted,
        })
    }

    /// Run `VACUUM` to reclaim disk space after a dedupe.
    #[cfg_attr(not(feature = "indexer"), allow(dead_code))]
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute_batch("VACUUM;")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_open_creates_schema() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Check that tables exist
        let table_count: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('meta', 'package_versions')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 2);
    }

    #[test]
    fn test_database_meta_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Initially no value
        assert!(db.get_meta("test_key").unwrap().is_none());

        // Set and get
        db.set_meta("test_key", "test_value").unwrap();
        assert_eq!(
            db.get_meta("test_key").unwrap(),
            Some("test_value".to_string())
        );

        // Update
        db.set_meta("test_key", "new_value").unwrap();
        assert_eq!(
            db.get_meta("test_key").unwrap(),
            Some("new_value".to_string())
        );
    }

    #[test]
    fn test_database_open_readonly_missing_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("nonexistent.db");
        let result = Database::open_readonly(&db_path);
        assert!(matches!(result, Err(NxvError::NoIndex)));
    }

    #[test]
    fn test_schema_versioning() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let version = db.get_meta("schema_version").unwrap();
        assert_eq!(version, Some(SCHEMA_VERSION.to_string()));
    }

    #[test]
    fn test_batch_insert() {
        use chrono::Utc;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let now = Utc::now();
        let packages = vec![
            PackageVersion {
                id: 0,
                name: "python".to_string(),
                version: "3.11.0".to_string(),
                first_commit_hash: "abc1234567890".to_string(),
                first_commit_date: now,
                last_commit_hash: "def1234567890".to_string(),
                last_commit_date: now,
                attribute_path: "python311".to_string(),
                description: Some("Python interpreter".to_string()),
                license: None,
                homepage: None,
                maintainers: None,
                platforms: None,
                source_path: None,
                known_vulnerabilities: None,
            },
            PackageVersion {
                id: 0,
                name: "nodejs".to_string(),
                version: "20.0.0".to_string(),
                first_commit_hash: "ghi1234567890".to_string(),
                first_commit_date: now,
                last_commit_hash: "jkl1234567890".to_string(),
                last_commit_date: now,
                attribute_path: "nodejs_20".to_string(),
                description: Some("Node.js runtime".to_string()),
                license: None,
                homepage: None,
                maintainers: None,
                platforms: None,
                source_path: None,
                known_vulnerabilities: None,
            },
        ];

        let inserted = db.upsert_observations(&packages).unwrap();
        assert_eq!(inserted, 2);

        // Verify data was inserted
        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_upsert_widens_range_and_updates_metadata() {
        use chrono::Utc;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let t0 = Utc::now();
        let mut pkg = PackageVersion {
            id: 0,
            name: "python".to_string(),
            version: "3.11.0".to_string(),
            first_commit_hash: "first_commit".to_string(),
            first_commit_date: t0,
            last_commit_hash: "last_commit_a".to_string(),
            last_commit_date: t0,
            attribute_path: "python311".to_string(),
            description: Some("Python interpreter".to_string()),
            license: None,
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: Some("pkgs/development/interpreters/python".to_string()),
            known_vulnerabilities: None,
        };

        let written1 = db.upsert_observations(std::slice::from_ref(&pkg)).unwrap();
        assert_eq!(written1, 1);

        // A newer observation extends the range end and refreshes metadata,
        // without creating a duplicate row.
        let t1 = t0 + chrono::Duration::days(30);
        pkg.first_commit_hash = "newer_first".to_string();
        pkg.first_commit_date = t1;
        pkg.last_commit_hash = "last_commit_b".to_string();
        pkg.last_commit_date = t1;
        pkg.description = Some("Python 3.11 interpreter".to_string());
        // source_path arriving as None must not clobber the existing value.
        pkg.source_path = None;

        let written2 = db.upsert_observations(std::slice::from_ref(&pkg)).unwrap();
        assert_eq!(written2, 1);

        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1, "upsert must not create a duplicate row");

        let (first_hash, last_hash, last_ts, description, source_path): (
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
        ) = db
            .conn
            .query_row(
                "SELECT first_commit_hash, last_commit_hash, last_commit_date, description, source_path FROM package_versions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            first_hash, "first_commit",
            "first bound must not move forward"
        );
        assert_eq!(last_hash, "last_commit_b");
        assert_eq!(last_ts, t1.timestamp());
        assert_eq!(description.as_deref(), Some("Python 3.11 interpreter"));
        assert_eq!(
            source_path.as_deref(),
            Some("pkgs/development/interpreters/python"),
            "source_path must be sticky (preserved when new value is NULL)"
        );
    }

    #[test]
    fn test_upsert_out_of_order_older_observation_widens_back_without_clobbering() {
        use chrono::Utc;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let t1 = Utc::now();
        let t0 = t1 - chrono::Duration::days(90);

        let newer = PackageVersion {
            id: 0,
            name: "firefox".to_string(),
            version: "100.0".to_string(),
            first_commit_hash: "newer_commit".to_string(),
            first_commit_date: t1,
            last_commit_hash: "newer_commit".to_string(),
            last_commit_date: t1,
            attribute_path: "firefox".to_string(),
            description: Some("new description".to_string()),
            license: None,
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: Some("pkgs/by-name/fi/firefox/package.nix".to_string()),
            known_vulnerabilities: None,
        };
        let older = PackageVersion {
            first_commit_hash: "older_commit".to_string(),
            first_commit_date: t0,
            last_commit_hash: "older_commit".to_string(),
            last_commit_date: t0,
            description: Some("stale description".to_string()),
            source_path: None,
            ..newer.clone()
        };

        // Ingest newest first (parallel/out-of-order ingestion), then the older
        // snapshot arrives late.
        db.upsert_observations(std::slice::from_ref(&newer))
            .unwrap();
        db.upsert_observations(std::slice::from_ref(&older))
            .unwrap();

        let (first_hash, first_ts, last_hash, last_ts, description): (
            String,
            i64,
            String,
            i64,
            Option<String>,
        ) = db
            .conn
            .query_row(
                "SELECT first_commit_hash, first_commit_date, last_commit_hash, last_commit_date, description FROM package_versions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            first_hash, "older_commit",
            "first bound must widen backwards"
        );
        assert_eq!(first_ts, t0.timestamp());
        assert_eq!(last_hash, "newer_commit", "last bound must not shrink");
        assert_eq!(last_ts, t1.timestamp());
        assert_eq!(
            description.as_deref(),
            Some("new description"),
            "late-arriving older metadata must not clobber newer metadata"
        );
    }

    #[test]
    fn test_fts5_trigger_sync() {
        use chrono::Utc;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let now = Utc::now();
        let pkg = PackageVersion {
            id: 0,
            name: "python".to_string(),
            version: "3.11.0".to_string(),
            first_commit_hash: "abc1234567890".to_string(),
            first_commit_date: now,
            last_commit_hash: "def1234567890".to_string(),
            last_commit_date: now,
            attribute_path: "python311".to_string(),
            description: Some("Python interpreter for scripting".to_string()),
            license: None,
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: None,
            known_vulnerabilities: None,
        };

        db.upsert_observations(&[pkg]).unwrap();

        // FTS5 should be searchable
        let fts_count: i32 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM package_versions_fts WHERE package_versions_fts MATCH 'scripting'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 1);
    }

    #[test]
    fn test_batch_insert_10k_performance() {
        use chrono::Utc;
        use std::time::Instant;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let now = Utc::now();
        let packages: Vec<PackageVersion> = (0..10_000)
            .map(|i| PackageVersion {
                id: 0,
                name: format!("package{}", i),
                version: format!("1.0.{}", i),
                first_commit_hash: format!("abc{:040}", i),
                first_commit_date: now,
                last_commit_hash: format!("def{:040}", i),
                last_commit_date: now,
                attribute_path: format!("packages.package{}", i),
                description: Some(format!("Test package {}", i)),
                license: None,
                homepage: None,
                maintainers: None,
                platforms: None,
                source_path: None,
                known_vulnerabilities: None,
            })
            .collect();

        let start = Instant::now();
        let inserted = db.upsert_observations(&packages).unwrap();
        let duration = start.elapsed();

        assert_eq!(inserted, 10_000);
        // Debug builds in the Nix sandbox have been observed around 5–6s on
        // contended hardware; release is ~0.5s. Threshold is intentionally
        // generous — we're guarding against order-of-magnitude regressions
        // (e.g. accidentally-O(n²) inserts), not microbenchmarking.
        assert!(
            duration.as_millis() < 30_000,
            "Batch insert took {:?}, expected < 30 seconds",
            duration
        );

        // Verify data was inserted
        let count: i32 = db
            .conn
            .query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 10_000);
    }

    /// Build an on-disk v3-schema database (the old 3-column unique key, no
    /// releases table) with duplicate (attr, version) rows — the shape the
    /// v3->v4 migration must repair.
    fn create_v3_database(db_path: &std::path::Path) {
        let conn = Connection::open(db_path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE package_versions (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                first_commit_hash TEXT NOT NULL,
                first_commit_date INTEGER NOT NULL,
                last_commit_hash TEXT NOT NULL,
                last_commit_date INTEGER NOT NULL,
                attribute_path TEXT NOT NULL,
                description TEXT,
                license TEXT,
                homepage TEXT,
                maintainers TEXT,
                platforms TEXT,
                source_path TEXT,
                known_vulnerabilities TEXT,
                UNIQUE(attribute_path, version, first_commit_hash)
            );
            CREATE INDEX idx_packages_last_commit_hash ON package_versions(last_commit_hash);
            CREATE VIRTUAL TABLE package_versions_fts
            USING fts5(name, description, content=package_versions, content_rowid=id);
            CREATE TRIGGER package_versions_ai AFTER INSERT ON package_versions BEGIN
                INSERT INTO package_versions_fts(rowid, name, description)
                VALUES (new.id, new.name, new.description);
            END;
            CREATE TRIGGER package_versions_au AFTER UPDATE ON package_versions BEGIN
                INSERT INTO package_versions_fts(package_versions_fts, rowid, name, description)
                VALUES ('delete', old.id, old.name, old.description);
                INSERT INTO package_versions_fts(rowid, name, description)
                VALUES (new.id, new.name, new.description);
            END;
            INSERT INTO meta (key, value) VALUES ('schema_version', '3');

            -- Duplicate (firefox, 100.0) rows with different first hashes:
            -- the old incremental-indexer bloat.
            INSERT INTO package_versions
                (name, version, first_commit_hash, first_commit_date,
                 last_commit_hash, last_commit_date, attribute_path, description)
            VALUES
                ('firefox', '100.0', 'c0',  1000, 'c10', 1010, 'firefox', 'browser'),
                ('firefox', '100.0', 'c5',  1005, 'c20', 1020, 'firefox', 'browser'),
                ('firefox', '100.0', 'c15', 1015, 'c30', 1030, 'firefox', 'browser'),
                ('chromium', '90.0', 'x0',  1000, 'x1',  1001, 'chromium', NULL);
            "#,
        )
        .unwrap();
    }

    #[test]
    fn test_migration_v3_to_v4_dedupes_and_tightens_unique_key() {
        use chrono::TimeZone;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        create_v3_database(&db_path);

        // Opening the v3 DB runs the migration.
        let mut db = Database::open(&db_path).unwrap();

        assert_eq!(db.get_meta("schema_version").unwrap().as_deref(), Some("4"));

        // Duplicates collapsed; survivor carries the widened range.
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);

        let (first_hash, first_ts, last_hash, last_ts): (String, i64, String, i64) = db
            .conn
            .query_row(
                "SELECT first_commit_hash, first_commit_date, last_commit_hash, last_commit_date \
                   FROM package_versions WHERE attribute_path = 'firefox'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(first_hash, "c0", "earliest first_commit_hash must survive");
        assert_eq!(first_ts, 1000);
        assert_eq!(last_hash, "c30", "latest last_commit_hash must win");
        assert_eq!(last_ts, 1030);

        // The tightened unique key must exist and the v4 upsert must prepare
        // and execute against the migrated DB.
        let has_uq: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='index' AND name='uq_attr_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_uq, "migration must create uq_attr_version");

        let pkg = PackageVersion {
            id: 0,
            name: "firefox".to_string(),
            version: "100.0".to_string(),
            first_commit_hash: "c40".to_string(),
            first_commit_date: chrono::Utc.timestamp_opt(1040, 0).unwrap(),
            last_commit_hash: "c40".to_string(),
            last_commit_date: chrono::Utc.timestamp_opt(1040, 0).unwrap(),
            attribute_path: "firefox".to_string(),
            description: Some("browser".to_string()),
            license: None,
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: None,
            known_vulnerabilities: None,
        };
        db.upsert_observations(&[pkg]).unwrap();

        let (last_hash, count): (String, i64) = db
            .conn
            .query_row(
                "SELECT last_commit_hash, (SELECT COUNT(*) FROM package_versions WHERE attribute_path='firefox') \
                   FROM package_versions WHERE attribute_path = 'firefox'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(last_hash, "c40");
        assert_eq!(
            count, 1,
            "upsert against migrated DB must widen, not duplicate"
        );

        // The releases ledger must exist after migration.
        let has_releases: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='releases'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(has_releases);
    }

    #[test]
    fn test_fts_triggers_missing_detection_and_heal() {
        use chrono::Utc;

        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();
        assert!(!db.fts_triggers_missing().unwrap());

        // Simulate a bulk run dying between drop_fts_triggers and
        // rebuild_fts: rows written in that window never reach FTS.
        db.drop_fts_triggers().unwrap();
        let now = Utc::now();
        db.upsert_observations(&[PackageVersion {
            id: 0,
            name: "ripgrep".to_string(),
            version: "14.0.0".to_string(),
            first_commit_hash: "a".repeat(40),
            first_commit_date: now,
            last_commit_hash: "a".repeat(40),
            last_commit_date: now,
            attribute_path: "ripgrep".to_string(),
            description: Some("fast grep replacement".to_string()),
            license: None,
            homepage: None,
            maintainers: None,
            platforms: None,
            source_path: None,
            known_vulnerabilities: None,
        }])
        .unwrap();
        drop(db);

        // Reopen (as the next run would) and heal.
        let db = Database::open(&db_path).unwrap();
        assert!(
            db.fts_triggers_missing().unwrap(),
            "must detect the broken state"
        );
        db.rebuild_fts().unwrap();
        assert!(!db.fts_triggers_missing().unwrap());

        let fts_hits: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM package_versions_fts WHERE package_versions_fts MATCH 'grep'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            fts_hits, 1,
            "rows written while triggers were absent must be re-indexed"
        );
    }

    #[test]
    fn test_dedupe_noop_on_empty_db() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut db = Database::open(&db_path).unwrap();

        let stats = db.dedupe_ranges(false).unwrap();
        assert_eq!(stats.groups_total, 0);
        assert_eq!(stats.rows_before, 0);
        assert_eq!(stats.rows_after, 0);
    }
}

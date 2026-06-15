//! Benchmarks for v4 search/query performance.
//!
//! The production v4 index is much larger than the old git-walked index:
//! it stores one row per `(attribute_path, version)` and includes nested
//! package sets. These benchmarks model the hot paths that regressed with that
//! cardinality: exact package lookup, package+version prefix search,
//! completion, stats, and FTS.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rusqlite::Connection;
use std::hint::black_box;
use tempfile::tempdir;

fn prefix_upper_bound(prefix: &str) -> String {
    let mut chars: Vec<char> = prefix.chars().collect();
    let last = chars.pop().expect("non-empty benchmark prefix");
    chars.push(char::from_u32(last as u32 + 1).expect("benchmark prefix upper bound"));
    chars.into_iter().collect()
}

/// Create a v4-shaped benchmark database with many nested attrs and versions.
fn create_benchmark_db(
    num_attrs: usize,
    versions_per_attr: usize,
) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("bench.db");

    let mut conn = Connection::open(&db_path).unwrap();
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

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
            UNIQUE(attribute_path, version),
            CHECK(first_commit_date <= last_commit_date)
        );

        CREATE TABLE package_attrs (
            attribute_path TEXT PRIMARY KEY
        ) WITHOUT ROWID;

        CREATE INDEX idx_packages_name ON package_versions(name);
        CREATE INDEX idx_packages_name_version ON package_versions(name, version, first_commit_date);
        CREATE INDEX idx_packages_attr ON package_versions(attribute_path);
        CREATE INDEX idx_packages_first_date ON package_versions(first_commit_date DESC);
        CREATE INDEX idx_packages_last_date ON package_versions(last_commit_date DESC);
        CREATE INDEX idx_version_vulnerabilities ON package_versions(version)
            WHERE known_vulnerabilities IS NOT NULL
              AND known_vulnerabilities != ''
              AND known_vulnerabilities != '[]'
              AND known_vulnerabilities != 'null';

        CREATE VIRTUAL TABLE package_versions_fts
        USING fts5(name, description, content=package_versions, content_rowid=id);

        CREATE TRIGGER package_versions_ai AFTER INSERT ON package_versions BEGIN
            INSERT INTO package_versions_fts(rowid, name, description)
            VALUES (new.id, new.name, new.description);
        END;
        "#,
    )
    .unwrap();

    let tx = conn.transaction().unwrap();
    {
        let mut stmt = tx
            .prepare(
                r#"
            INSERT INTO package_versions
                (name, version, first_commit_hash, first_commit_date,
                 last_commit_hash, last_commit_date, attribute_path,
                 description, license, homepage, maintainers, platforms, source_path,
                 known_vulnerabilities)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            )
            .unwrap();

        for attr_idx in 0..num_attrs {
            let attr_path = if attr_idx % 4 == 0 {
                format!("python311Packages.pkg{attr_idx}")
            } else if attr_idx % 4 == 1 {
                format!("python312Packages.pkg{attr_idx}")
            } else if attr_idx % 4 == 2 {
                format!("haskellPackages.pkg{attr_idx}")
            } else {
                format!("package{attr_idx}")
            };
            let name = attr_path
                .rsplit('.')
                .next()
                .unwrap_or(&attr_path)
                .to_string();

            for version_idx in 0..versions_per_attr {
                let version = if attr_path.starts_with("python311") {
                    format!("3.11.{version_idx}")
                } else {
                    format!("1.{version_idx}.0")
                };
                let first = 1_700_000_000 + attr_idx as i64 + (version_idx as i64 * 10_000);
                stmt.execute(rusqlite::params![
                    name,
                    version,
                    format!("{:040x}", attr_idx * 1000 + version_idx),
                    first,
                    format!("{:040x}", attr_idx * 1000 + version_idx + 1),
                    first + 3600,
                    attr_path,
                    format!("Python benchmark package {attr_idx} with searchable text"),
                    r#"["MIT"]"#,
                    "https://example.com",
                    r#"["maintainer"]"#,
                    r#"["x86_64-linux"]"#,
                    format!("pkgs/by-name/{attr_idx}"),
                    Option::<String>::None,
                ])
                .unwrap();
            }
        }
    }
    tx.execute(
        "INSERT INTO package_attrs(attribute_path) SELECT DISTINCT attribute_path FROM package_versions",
        [],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO meta(key, value) VALUES
         ('stats_total_ranges', ?1),
         ('stats_unique_names', ?2),
         ('stats_unique_versions', ?3),
         ('stats_oldest_commit_date', '1700000000'),
         ('stats_newest_commit_date', '1800000000')",
        rusqlite::params![
            (num_attrs * versions_per_attr).to_string(),
            num_attrs.to_string(),
            versions_per_attr.to_string(),
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    (dir, db_path)
}

fn bench_exact_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("v4_exact_lookup");

    for size in [10_000, 50_000].iter() {
        let (_dir, db_path) = create_benchmark_db(*size, 8);
        let conn = Connection::open(&db_path).unwrap();
        let depth = "(LENGTH(attribute_path) - LENGTH(REPLACE(attribute_path, '.', '')))";

        group.bench_with_input(
            BenchmarkId::new("old_prefix_then_filter", size),
            size,
            |b, _| {
                b.iter(|| {
                    let sql = format!(
                        "SELECT * FROM package_versions WHERE attribute_path LIKE ? ESCAPE '\\' \
                     ORDER BY {depth} ASC, last_commit_date DESC LIMIT 5000"
                    );
                    let rows: Vec<String> = conn
                        .prepare_cached(&sql)
                        .unwrap()
                        .query_map(["python311Packages.pkg100%"], |row| {
                            row.get("attribute_path")
                        })
                        .unwrap()
                        .filter_map(Result::ok)
                        .filter(|attr| attr == "python311Packages.pkg100")
                        .collect();
                    black_box(rows)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("exact_attr", size), size, |b, _| {
            b.iter(|| {
                let rows: Vec<String> = conn
                    .prepare_cached(
                        "SELECT * FROM package_versions WHERE attribute_path = ? ORDER BY last_commit_date DESC",
                    )
                    .unwrap()
                    .query_map(["python311Packages.pkg100"], |row| row.get("attribute_path"))
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect();
                black_box(rows)
            });
        });
    }

    group.finish();
}

fn bench_prefix_version_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("v4_prefix_version_search");

    for size in [10_000, 50_000].iter() {
        let (_dir, db_path) = create_benchmark_db(*size, 8);
        let conn = Connection::open(&db_path).unwrap();
        let depth = "(LENGTH(attribute_path) - LENGTH(REPLACE(attribute_path, '.', '')))";
        let upper = prefix_upper_bound("python311");

        group.bench_with_input(BenchmarkId::new("like_prefix", size), size, |b, _| {
            b.iter(|| {
                let sql = format!(
                    "SELECT * FROM package_versions \
                     WHERE attribute_path LIKE ? ESCAPE '\\' AND version LIKE ? ESCAPE '\\' \
                     ORDER BY {depth} ASC, first_commit_date DESC LIMIT 5000"
                );
                let rows: Vec<String> = conn
                    .prepare_cached(&sql)
                    .unwrap()
                    .query_map(["python311%", "3.11%"], |row| row.get("attribute_path"))
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect();
                black_box(rows)
            });
        });

        group.bench_with_input(BenchmarkId::new("range_prefix", size), size, |b, _| {
            b.iter(|| {
                let sql = format!(
                    "SELECT * FROM package_versions \
                     WHERE attribute_path >= ? AND attribute_path < ? AND version LIKE ? ESCAPE '\\' \
                     ORDER BY {depth} ASC, first_commit_date DESC LIMIT 5000"
                );
                let rows: Vec<String> = conn
                    .prepare_cached(&sql)
                    .unwrap()
                    .query_map(rusqlite::params!["python311", upper, "3.11%"], |row| {
                        row.get("attribute_path")
                    })
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect();
                black_box(rows)
            });
        });
    }

    group.finish();
}

fn bench_completion(c: &mut Criterion) {
    let mut group = c.benchmark_group("v4_completion");

    for size in [10_000, 50_000].iter() {
        let (_dir, db_path) = create_benchmark_db(*size, 8);
        let conn = Connection::open(&db_path).unwrap();
        let upper = prefix_upper_bound("python");

        group.bench_with_input(
            BenchmarkId::new("distinct_package_versions", size),
            size,
            |b, _| {
                b.iter(|| {
                    let rows: Vec<String> = conn
                        .prepare_cached(
                            "SELECT DISTINCT attribute_path FROM package_versions \
                         WHERE attribute_path >= ? AND attribute_path < ? \
                         ORDER BY attribute_path LIMIT 100",
                        )
                        .unwrap()
                        .query_map(rusqlite::params!["python", upper], |row| row.get(0))
                        .unwrap()
                        .filter_map(Result::ok)
                        .collect();
                    black_box(rows)
                });
            },
        );

        group.bench_with_input(BenchmarkId::new("package_attrs", size), size, |b, _| {
            b.iter(|| {
                let rows: Vec<String> = conn
                    .prepare_cached(
                        "SELECT attribute_path FROM package_attrs \
                         WHERE attribute_path >= ? AND attribute_path < ? \
                         ORDER BY attribute_path LIMIT 100",
                    )
                    .unwrap()
                    .query_map(rusqlite::params!["python", upper], |row| row.get(0))
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect();
                black_box(rows)
            });
        });
    }

    group.finish();
}

fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("v4_stats");

    for size in [10_000, 50_000].iter() {
        let (_dir, db_path) = create_benchmark_db(*size, 8);
        let conn = Connection::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::new("live_scans", size), size, |b, _| {
            b.iter(|| {
                let row = conn
                    .prepare_cached(
                        "SELECT
                         (SELECT COUNT(*) FROM package_versions),
                         (SELECT COUNT(DISTINCT name) FROM package_versions),
                         (SELECT COUNT(DISTINCT version) FROM package_versions),
                         (SELECT MIN(first_commit_date) FROM package_versions),
                         (SELECT MAX(last_commit_date) FROM package_versions)",
                    )
                    .unwrap()
                    .query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
                    .unwrap();
                black_box(row)
            });
        });

        group.bench_with_input(BenchmarkId::new("meta_cache", size), size, |b, _| {
            b.iter(|| {
                let row = conn
                    .prepare_cached(
                        "SELECT
                         (SELECT value FROM meta WHERE key = 'stats_total_ranges'),
                         (SELECT value FROM meta WHERE key = 'stats_unique_names'),
                         (SELECT value FROM meta WHERE key = 'stats_unique_versions'),
                         (SELECT value FROM meta WHERE key = 'stats_oldest_commit_date'),
                         (SELECT value FROM meta WHERE key = 'stats_newest_commit_date')",
                    )
                    .unwrap()
                    .query_row([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .unwrap();
                black_box(row)
            });
        });
    }

    group.finish();
}

fn bench_search_fts(c: &mut Criterion) {
    let mut group = c.benchmark_group("v4_fts");

    for size in [10_000, 50_000].iter() {
        let (_dir, db_path) = create_benchmark_db(*size, 8);
        let conn = Connection::open(&db_path).unwrap();

        group.bench_with_input(BenchmarkId::new("description", size), size, |b, _| {
            b.iter(|| {
                let rows: Vec<String> = conn
                    .prepare_cached(
                        r#"
                        SELECT pv.attribute_path FROM package_versions pv
                        JOIN package_versions_fts fts ON pv.id = fts.rowid
                        WHERE package_versions_fts MATCH ?
                        ORDER BY pv.last_commit_date DESC
                        LIMIT 5000
                        "#,
                    )
                    .unwrap()
                    .query_map(["python"], |row| row.get(0))
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect();
                black_box(rows)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_exact_lookup,
    bench_prefix_version_search,
    bench_completion,
    bench_stats,
    bench_search_fts
);
criterion_main!(benches);

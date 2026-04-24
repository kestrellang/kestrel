use glob::glob;
use rusqlite::{Connection, ErrorCode, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitCode, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use time::format_description::well_known::Rfc3339;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::{CommandExt, ExitStatusExt};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const ASYNC_CHILD_ENV: &str = "TRIAGE_ASYNC_CHILD";
const ASYNC_BUILD_ENV: &str = "TRIAGE_ASYNC_BUILD_ID";
const ASYNC_BINARY_ENV: &str = "TRIAGE_ASYNC_BINARY";
const ASYNC_PATTERN_ENV: &str = "TRIAGE_ASYNC_PATTERN";
const ASYNC_JOBS_ENV: &str = "TRIAGE_ASYNC_JOBS";
const ASYNC_JSON_ENV: &str = "TRIAGE_ASYNC_JSON";
const ASYNC_JQ_ENV: &str = "TRIAGE_ASYNC_JQ";
const ASYNC_STRATEGY_ENV: &str = "TRIAGE_ASYNC_STRATEGY";
const ASYNC_BATCH_SIZE_ENV: &str = "TRIAGE_ASYNC_BATCH_SIZE";

const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    r#"
CREATE TABLE IF NOT EXISTS build (
    id TEXT PRIMARY KEY NOT NULL,
    binary_hash TEXT NOT NULL UNIQUE,
    commit_sha TEXT NOT NULL,
    branch TEXT,
    dirty INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS test (
    id TEXT PRIMARY KEY NOT NULL,
    path TEXT NOT NULL UNIQUE,
    first_seen_build TEXT NOT NULL,
    last_seen_build TEXT,
    removed_at TEXT,
    quarantined INTEGER NOT NULL DEFAULT 0,
    skip_reason TEXT,
    FOREIGN KEY(first_seen_build) REFERENCES build(id),
    FOREIGN KEY(last_seen_build) REFERENCES build(id)
);

CREATE TABLE IF NOT EXISTS test_run (
    id TEXT PRIMARY KEY NOT NULL,
    build_id TEXT NOT NULL,
    test_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    status TEXT NOT NULL,
    exit_code INTEGER,
    duration_ms INTEGER,
    failure_message TEXT,
    worker_id TEXT,
    heartbeat_at TEXT,
    FOREIGN KEY(build_id) REFERENCES build(id),
    FOREIGN KEY(test_id) REFERENCES test(id),
    UNIQUE(build_id, test_id)
);

CREATE INDEX IF NOT EXISTS idx_test_live_path
    ON test(path)
    WHERE removed_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_test_run_build_status
    ON test_run(build_id, status);

CREATE INDEX IF NOT EXISTS idx_test_run_test_created
    ON test_run(test_id, created_at);
"#,
)];

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        },
    }
}

fn real_main() -> Result<ExitCode> {
    let cli = Cli::parse(env::args_os().skip(1))?;

    if matches!(cli.action, Action::Help) {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }
    if matches!(cli.action, Action::Version) {
        println!("triage {}", env!("CARGO_PKG_VERSION"));
        return Ok(ExitCode::SUCCESS);
    }

    let ctx = AppContext::load(&cli)?;

    match &cli.action {
        Action::Run { pattern } => run_command(&ctx, &cli, pattern),
        Action::Status { build_id } => status_command(&ctx, &cli, build_id.as_deref()),
        Action::History { test } => history_command(&ctx, &cli, test),
        Action::Builds => builds_command(&ctx, &cli),
        Action::Quarantine { test, reason } => quarantine_command(&ctx, &cli, test, reason),
        Action::Unquarantine { test } => unquarantine_command(&ctx, &cli, test),
        Action::Cancel { build_id } => cancel_command(&ctx, &cli, build_id),
        Action::Help | Action::Version => unreachable!(),
    }
}

#[derive(Debug)]
struct Cli {
    action: Action,
    jobs: Option<usize>,
    db: Option<PathBuf>,
    binary: Option<PathBuf>,
    json: bool,
    jq: Option<String>,
    async_run: bool,
    show_failures: bool,
    show_messages: bool,
    /// Execution strategy override. `None` → fall back to config default.
    strategy: Option<Strategy>,
    /// Per-worker batch size override (only meaningful for `Strategy::Batch`).
    batch_size: Option<usize>,
}

/// How each worker runs tests.
///
/// * `Isolated` spawns one `file_tests` subprocess per test (the pre-Phase-B
///   behavior). Maximally robust — every test starts from a clean address
///   space — but pays the subprocess + stdlib-init cost for every test.
/// * `Batch` claims a chunk of tests at a time and passes them to a single
///   `file_tests --names-file` invocation. Amortizes stdlib init across the
///   batch; a panic in one test no longer taints the others because the
///   harness wraps each test in `catch_unwind`. A hard crash (abort/signal)
///   still drops the whole batch — the in-flight test is attributed as
///   `crashed`/`timed_out` and the remainder is requeued for other workers
///   to pick up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Strategy {
    Isolated,
    Batch,
}

impl Default for Strategy {
    fn default() -> Self {
        Strategy::Batch
    }
}

impl Strategy {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "isolated" => Ok(Strategy::Isolated),
            "batch" => Ok(Strategy::Batch),
            other => Err(boxed(format!(
                "unknown --strategy `{other}` (expected `isolated` or `batch`)"
            ))),
        }
    }
}

#[derive(Debug)]
enum Action {
    Run { pattern: String },
    Status { build_id: Option<String> },
    History { test: String },
    Builds,
    Quarantine { test: String, reason: String },
    Unquarantine { test: String },
    Cancel { build_id: String },
    Help,
    Version,
}

impl Cli {
    fn parse<I>(args: I) -> Result<Self>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut jobs = None;
        let mut db = None;
        let mut binary = None;
        let mut json = false;
        let mut jq = None;
        let mut async_run = false;
        let mut show_failures = false;
        let mut show_messages = false;
        let mut strategy = None;
        let mut batch_size = None;
        let mut positional = Vec::new();

        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let s = arg
                .to_str()
                .ok_or_else(|| boxed("arguments must be valid UTF-8"))?;

            match s {
                "-h" | "--help" => {
                    return Ok(Self {
                        action: Action::Help,
                        jobs,
                        db,
                        binary,
                        json,
                        jq,
                        async_run,
                        show_failures,
                        show_messages,
                        strategy,
                        batch_size,
                    });
                },
                "-V" | "--version" => {
                    return Ok(Self {
                        action: Action::Version,
                        jobs,
                        db,
                        binary,
                        json,
                        jq,
                        async_run,
                        show_failures,
                        show_messages,
                        strategy,
                        batch_size,
                    });
                },
                "-a" | "--async" => async_run = true,
                "--json" => json = true,
                "--failures" => show_failures = true,
                "--messages" => show_messages = true,
                "-j" | "--jobs" => {
                    let value = next_arg(&mut iter, s)?;
                    jobs = Some(parse_usize(&value, s)?);
                },
                "--db" => {
                    let value = next_arg(&mut iter, s)?;
                    db = Some(PathBuf::from(value));
                },
                "--binary" => {
                    let value = next_arg(&mut iter, s)?;
                    binary = Some(PathBuf::from(value));
                },
                "--jq" => {
                    let value = next_arg(&mut iter, s)?;
                    jq = Some(value);
                    json = true;
                },
                "--strategy" => {
                    let value = next_arg(&mut iter, s)?;
                    strategy = Some(Strategy::parse(&value)?);
                },
                "--batch-size" => {
                    let value = next_arg(&mut iter, s)?;
                    batch_size = Some(parse_usize(&value, s)?);
                },
                _ if s.starts_with("--strategy=") => {
                    strategy = Some(Strategy::parse(&s["--strategy=".len()..])?);
                },
                _ if s.starts_with("--batch-size=") => {
                    batch_size = Some(parse_usize(&s["--batch-size=".len()..], "--batch-size")?);
                },
                _ if s.starts_with("--jobs=") => {
                    jobs = Some(parse_usize(&s["--jobs=".len()..], "--jobs")?);
                },
                _ if s.starts_with("-j") && s.len() > 2 => {
                    jobs = Some(parse_usize(&s[2..], "-j")?);
                },
                _ if s.starts_with("--db=") => {
                    db = Some(PathBuf::from(&s["--db=".len()..]));
                },
                _ if s.starts_with("--binary=") => {
                    binary = Some(PathBuf::from(&s["--binary=".len()..]));
                },
                _ if s.starts_with("--jq=") => {
                    jq = Some(s["--jq=".len()..].to_string());
                    json = true;
                },
                _ if s.starts_with('-') => {
                    return Err(boxed(format!("unknown flag `{s}`")));
                },
                _ => positional.push(s.to_string()),
            }
        }

        let action = parse_action(positional)?;
        if async_run && !matches!(action, Action::Run { .. }) {
            return Err(boxed("--async only applies to the run command"));
        }
        if show_messages {
            show_failures = true;
        }
        if (show_failures || show_messages) && !matches!(action, Action::Status { .. }) {
            return Err(boxed("--failures and --messages only apply to status"));
        }

        if (strategy.is_some() || batch_size.is_some()) && !matches!(action, Action::Run { .. }) {
            return Err(boxed(
                "--strategy and --batch-size only apply to the run command",
            ));
        }

        Ok(Self {
            action,
            jobs,
            db,
            binary,
            json,
            jq,
            async_run,
            show_failures,
            show_messages,
            strategy,
            batch_size,
        })
    }
}

fn next_arg<I>(iter: &mut I, flag: &str) -> Result<String>
where
    I: Iterator<Item = OsString>,
{
    let value = iter
        .next()
        .ok_or_else(|| boxed(format!("{flag} requires a value")))?;
    value
        .into_string()
        .map_err(|_| boxed(format!("{flag} value must be valid UTF-8")))
}

fn parse_usize(value: &str, flag: &str) -> Result<usize> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| boxed(format!("{flag} requires a positive integer")))?;
    if parsed == 0 {
        return Err(boxed(format!("{flag} must be greater than zero")));
    }
    Ok(parsed)
}

fn parse_action(positional: Vec<String>) -> Result<Action> {
    let Some(first) = positional.first() else {
        return Ok(Action::Run {
            pattern: "*".to_string(),
        });
    };

    match first.as_str() {
        "status" => {
            if positional.len() > 2 {
                return Err(boxed("usage: triage status [build_id]"));
            }
            Ok(Action::Status {
                build_id: positional.get(1).cloned(),
            })
        },
        "history" => {
            if positional.len() != 2 {
                return Err(boxed("usage: triage history <test>"));
            }
            Ok(Action::History {
                test: positional[1].clone(),
            })
        },
        "builds" => {
            if positional.len() != 1 {
                return Err(boxed("usage: triage builds"));
            }
            Ok(Action::Builds)
        },
        "quarantine" => {
            if positional.len() < 3 {
                return Err(boxed("usage: triage quarantine <test> <reason>"));
            }
            Ok(Action::Quarantine {
                test: positional[1].clone(),
                reason: positional[2..].join(" "),
            })
        },
        "unquarantine" => {
            if positional.len() != 2 {
                return Err(boxed("usage: triage unquarantine <test>"));
            }
            Ok(Action::Unquarantine {
                test: positional[1].clone(),
            })
        },
        "cancel" => {
            if positional.len() != 2 {
                return Err(boxed("usage: triage cancel <build_id>"));
            }
            Ok(Action::Cancel {
                build_id: positional[1].clone(),
            })
        },
        _ => {
            if positional.len() != 1 {
                return Err(boxed("run accepts at most one pattern: triage [pattern]"));
            }
            Ok(Action::Run {
                pattern: first.clone(),
            })
        },
    }
}

fn print_help() {
    println!(
        "triage {}\n\nUSAGE:\n  triage [pattern] [flags]\n  triage [pattern] --async\n  triage status [build_id] [--failures] [--messages]\n  triage history <test>\n  triage builds\n  triage quarantine <test> <reason>\n  triage unquarantine <test>\n  triage cancel <build_id>\n\nFLAGS:\n  -j, --jobs N        Worker parallelism\n      --db PATH       SQLite database path (env: TRIAGE_DB)\n      --binary PATH   Explicit file_tests executable\n      --strategy S    Worker strategy: `batch` (default) or `isolated`\n      --batch-size N  Tests per subprocess when --strategy=batch (default 16)\n      --json          Emit JSON / NDJSON\n      --jq EXPR       Filter JSON output through jq\n      --failures      Include failed/problem test rows in status output\n      --messages      Include failure messages in status output; implies --failures\n  -a, --async         Run in a detached worker process\n  -h, --help          Print help\n  -V, --version       Print version",
        env!("CARGO_PKG_VERSION")
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Config {
    package: String,
    binary_glob: String,
    harness_prefix: String,
    test_extension: String,
    binary_cwd: String,
    build_command: Vec<String>,
    stall_threshold_seconds: u64,
    jobs: usize,
    #[serde(default)]
    strategy: Strategy,
    #[serde(default = "default_batch_size")]
    batch_size: usize,
}

fn default_batch_size() -> usize {
    16
}

impl Default for Config {
    fn default() -> Self {
        Self {
            package: "kestrel-test-suite".to_string(),
            binary_glob: "file_tests-*".to_string(),
            harness_prefix: "run_ks_test::".to_string(),
            test_extension: ".ks".to_string(),
            binary_cwd: "lib/kestrel-test-suite".to_string(),
            build_command: vec![
                "cargo".to_string(),
                "test".to_string(),
                "-p".to_string(),
                "kestrel-test-suite".to_string(),
                "--release".to_string(),
            ],
            stall_threshold_seconds: 30,
            jobs: 1,
            strategy: Strategy::Batch,
            batch_size: default_batch_size(),
        }
    }
}

#[derive(Debug, Clone)]
struct AppContext {
    repo_root: PathBuf,
    triage_dir: PathBuf,
    db_path: PathBuf,
    config: Config,
}

impl AppContext {
    fn load(cli: &Cli) -> Result<Self> {
        let cwd = env::current_dir()?;
        let repo_root = discover_repo_root(&cwd)?;
        let db_path = cli
            .db
            .clone()
            .or_else(|| env::var_os("TRIAGE_DB").map(PathBuf::from))
            .map(|path| absolutize(&cwd, path))
            .unwrap_or_else(|| repo_root.join(".triage").join("triage.db"));

        let triage_dir = db_path
            .parent()
            .ok_or_else(|| boxed("database path must have a parent directory"))?
            .to_path_buf();

        fs::create_dir_all(&triage_dir)?;
        fs::create_dir_all(triage_dir.join("logs"))?;
        fs::create_dir_all(triage_dir.join("binaries"))?;
        fs::create_dir_all(triage_dir.join("runs"))?;

        let gitignore = triage_dir.join(".gitignore");
        if !gitignore.exists() {
            fs::write(&gitignore, "*\n")?;
        }

        let config_path = triage_dir.join("config.toml");
        if !config_path.exists() {
            let config_text = toml::to_string_pretty(&Config::default())?;
            fs::write(&config_path, config_text)?;
        }
        let config_text = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&config_text)?;

        let ctx = Self {
            repo_root,
            triage_dir,
            db_path,
            config,
        };
        ctx.sweep_binaries()?;
        let conn = ctx.open_db()?;
        apply_migrations(&conn)?;
        Ok(ctx)
    }

    fn logs_dir(&self) -> PathBuf {
        self.triage_dir.join("logs")
    }

    fn binaries_dir(&self) -> PathBuf {
        self.triage_dir.join("binaries")
    }

    fn runs_dir(&self) -> PathBuf {
        self.triage_dir.join("runs")
    }

    fn binary_cwd(&self) -> PathBuf {
        self.repo_root.join(&self.config.binary_cwd)
    }

    fn open_db(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        configure_connection(&conn)?;
        Ok(conn)
    }

    fn sweep_binaries(&self) -> Result<()> {
        let dir = self.binaries_dir();
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let pid_path = path.join("pid");
            let stale = match fs::read_to_string(&pid_path) {
                Ok(pid_text) => match pid_text.trim().parse::<u32>() {
                    Ok(pid) => !process_is_alive(pid),
                    Err(_) => true,
                },
                Err(_) => true,
            };
            if stale {
                fs::remove_dir_all(path)?;
            }
        }

        Ok(())
    }
}

fn discover_repo_root(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".triage").is_dir() {
            return Ok(current);
        }
        if current.join(".git").exists() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(boxed("could not find a repo root (.triage or .git)"));
        }
    }
}

fn absolutize(cwd: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn configure_connection(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(30))?;
    retry_sqlite(|| conn.pragma_update(None, "foreign_keys", "ON"))?;
    retry_sqlite(|| conn.pragma_update(None, "journal_mode", "WAL"))?;
    Ok(())
}

fn retry_sqlite<T>(mut op: impl FnMut() -> rusqlite::Result<T>) -> rusqlite::Result<T> {
    let started = Instant::now();
    let mut delay = Duration::from_millis(10);

    loop {
        match op() {
            Ok(value) => return Ok(value),
            Err(err) if sqlite_lock_error(&err) && started.elapsed() < Duration::from_secs(30) => {
                thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(250));
            },
            Err(err) => return Err(err),
        }
    }
}

fn sqlite_lock_error(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if matches!(sqlite_err.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

fn apply_migrations(conn: &Connection) -> Result<()> {
    retry_sqlite(|| conn.execute_batch("BEGIN IMMEDIATE;"))?;

    let migration_result = (|| -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
             version INTEGER PRIMARY KEY,
             applied_at TEXT NOT NULL
         );",
        )?;
        for (version, sql) in MIGRATIONS {
            let exists: Option<i64> = conn
                .query_row(
                    "SELECT version FROM schema_migrations WHERE version = ?1",
                    params![version],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                conn.execute_batch(sql)?;
                conn.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                    params![version, now_string()?],
                )?;
            }
        }
        Ok(())
    })();

    match migration_result {
        Ok(()) => {
            retry_sqlite(|| conn.execute_batch("COMMIT;"))?;
            Ok(())
        },
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        },
    }
}

fn run_command(ctx: &AppContext, cli: &Cli, pattern: &str) -> Result<ExitCode> {
    if env::var_os(ASYNC_CHILD_ENV).is_some() {
        let build_id = env::var(ASYNC_BUILD_ENV)?;
        let binary = PathBuf::from(env::var(ASYNC_BINARY_ENV)?);
        let pattern = env::var(ASYNC_PATTERN_ENV)?;
        let jobs = env::var(ASYNC_JOBS_ENV)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or_else(|| jobs_for(cli, &ctx.config));
        let json = env::var_os(ASYNC_JSON_ENV).is_some();
        let jq = env::var(ASYNC_JQ_ENV).ok();
        let strategy = env::var(ASYNC_STRATEGY_ENV)
            .ok()
            .and_then(|v| Strategy::parse(&v).ok())
            .unwrap_or_else(|| strategy_for(cli, &ctx.config));
        let batch_size = env::var(ASYNC_BATCH_SIZE_ENV)
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or_else(|| batch_size_for(cli, &ctx.config));
        return execute_scheduled_run(
            ctx,
            &build_id,
            &binary,
            &pattern,
            jobs,
            OutputOptions { json, jq },
            ExecOptions {
                strategy,
                batch_size,
            },
        );
    }

    let conn = ctx.open_db()?;
    let build = match build_project(ctx, cli, &conn)? {
        BuildOutcome::Ready(build) => build,
        BuildOutcome::Failed(code) => return Ok(code),
    };

    let listed = discover_tests(ctx, &build.binary_path)?;
    sync_tests(&conn, &build.id, &listed)?;
    let scheduled = schedule_tests(&conn, &build.id, pattern)?;

    if cli.async_run {
        return spawn_async(ctx, cli, pattern, &build, &scheduled);
    }

    let jobs = jobs_for(cli, &ctx.config);
    let output = OutputOptions {
        json: cli.json,
        jq: cli.jq.clone(),
    };
    let exec = ExecOptions {
        strategy: strategy_for(cli, &ctx.config),
        batch_size: batch_size_for(cli, &ctx.config),
    };
    execute_scheduled_run(
        ctx,
        &build.id,
        &build.binary_path,
        pattern,
        jobs,
        output,
        exec,
    )
}

#[derive(Debug)]
enum BuildOutcome {
    Ready(BuildInfo),
    Failed(ExitCode),
}

#[derive(Debug, Clone)]
struct BuildInfo {
    id: String,
    binary_path: PathBuf,
}

fn build_project(ctx: &AppContext, cli: &Cli, conn: &Connection) -> Result<BuildOutcome> {
    if let Some(binary) = &cli.binary {
        let binary_path = absolutize(&env::current_dir()?, binary.clone());
        let build = resolve_build_row(ctx, conn, &binary_path)?;
        return Ok(BuildOutcome::Ready(build));
    }

    let mut command = ctx.config.build_command.clone();
    if command.is_empty() {
        return Err(boxed("config build_command cannot be empty"));
    }
    if !command.iter().any(|arg| arg == "--no-run") {
        command.push("--no-run".to_string());
    }

    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(&ctx.repo_root)
        .output()?;

    if !output.status.success() {
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        let code = output.status.code().unwrap_or(1);
        return Ok(BuildOutcome::Failed(exit_code(code)));
    }

    let binary_path = find_test_binary(ctx)?;
    let build = resolve_build_row(ctx, conn, &binary_path)?;
    Ok(BuildOutcome::Ready(build))
}

fn resolve_build_row(ctx: &AppContext, conn: &Connection, binary_path: &Path) -> Result<BuildInfo> {
    let binary_hash = sha256_file(binary_path)?;
    let now = now_string()?;
    let id = Uuid::new_v4().to_string();
    let git = git_info(&ctx.repo_root);

    conn.execute(
        "INSERT OR IGNORE INTO build
         (id, binary_hash, commit_sha, branch, dirty, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            binary_hash,
            git.commit_sha,
            git.branch,
            git.dirty as i64,
            now
        ],
    )?;

    let build_id: String = conn.query_row(
        "SELECT id FROM build WHERE binary_hash = ?1",
        params![binary_hash],
        |row| row.get(0),
    )?;

    Ok(BuildInfo {
        id: build_id,
        binary_path: binary_path.to_path_buf(),
    })
}

fn find_test_binary(ctx: &AppContext) -> Result<PathBuf> {
    let pattern = ctx
        .repo_root
        .join("target")
        .join("release")
        .join("deps")
        .join(&ctx.config.binary_glob);
    let pattern = pattern
        .to_str()
        .ok_or_else(|| boxed("binary_glob path must be valid UTF-8"))?;

    let mut candidates = Vec::new();
    for entry in glob(pattern)? {
        let path = entry?;
        if !path.is_file() {
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "d") {
            continue;
        }
        if !is_executable(&path)? {
            continue;
        }
        let modified = fs::metadata(&path)?.modified()?;
        candidates.push((modified, path));
    }

    candidates
        .into_iter()
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
        .ok_or_else(|| boxed(format!("no test binary matched `{pattern}`")))
}

fn discover_tests(ctx: &AppContext, binary_path: &Path) -> Result<Vec<String>> {
    let output = Command::new(binary_path)
        .arg("--list")
        .arg("--format=terse")
        .current_dir(ctx.binary_cwd())
        .output()?;

    if !output.status.success() {
        return Err(boxed(format!(
            "test discovery failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut paths = Vec::new();
    for line in stdout.lines() {
        let name = line.split_once(": ").map_or(line, |(name, _)| name);
        if let Some(path) = libtest_to_path(&ctx.config, name) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn sync_tests(conn: &Connection, build_id: &str, listed: &[String]) -> Result<()> {
    let now = now_string()?;
    retry_sqlite(|| conn.execute_batch("BEGIN IMMEDIATE;"))?;
    let result = (|| -> Result<()> {
        let listed_set: HashSet<&str> = listed.iter().map(String::as_str).collect();

        for path in listed {
            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM test WHERE path = ?1",
                    params![path],
                    |row| row.get(0),
                )
                .optional()?;
            match existing {
                Some(_) => {
                    conn.execute(
                        "UPDATE test
                         SET removed_at = NULL, last_seen_build = NULL
                         WHERE path = ?1 AND removed_at IS NOT NULL",
                        params![path],
                    )?;
                },
                None => {
                    conn.execute(
                        "INSERT INTO test
                         (id, path, first_seen_build, quarantined)
                         VALUES (?1, ?2, ?3, 0)",
                        params![Uuid::new_v4().to_string(), path, build_id],
                    )?;
                },
            }
        }

        let live_rows = {
            let mut stmt = conn.prepare("SELECT id, path FROM test WHERE removed_at IS NULL")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        for (test_id, path) in live_rows {
            if listed_set.contains(path.as_str()) {
                continue;
            }
            let last_seen = last_seen_build_for_test(conn, &test_id)?;
            conn.execute(
                "UPDATE test
                 SET removed_at = ?1, last_seen_build = ?2
                 WHERE id = ?3",
                params![now, last_seen, test_id],
            )?;
        }

        Ok(())
    })();

    match result {
        Ok(()) => {
            retry_sqlite(|| conn.execute_batch("COMMIT;"))?;
            Ok(())
        },
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        },
    }
}

fn last_seen_build_for_test(conn: &Connection, test_id: &str) -> Result<Option<String>> {
    let from_runs: Option<String> = conn
        .query_row(
            "SELECT build_id
             FROM test_run
             WHERE test_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
            params![test_id],
            |row| row.get(0),
        )
        .optional()?;
    if from_runs.is_some() {
        return Ok(from_runs);
    }
    conn.query_row(
        "SELECT first_seen_build FROM test WHERE id = ?1",
        params![test_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

#[derive(Debug, Clone)]
struct ScheduleStats {
    matched: usize,
    inserted: usize,
}

fn schedule_tests(conn: &Connection, build_id: &str, pattern: &str) -> Result<ScheduleStats> {
    let like = pattern_to_like(pattern);
    let rows = {
        let mut stmt = conn.prepare(
            "SELECT id, quarantined
             FROM test
             WHERE removed_at IS NULL AND path LIKE ?1 ESCAPE '\\'
             ORDER BY path",
        )?;
        let rows = stmt.query_map(params![like], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let mut inserted = 0;
    let now = now_string()?;
    retry_sqlite(|| conn.execute_batch("BEGIN IMMEDIATE;"))?;
    let result = (|| -> Result<()> {
        for (test_id, quarantined) in &rows {
            let run_id = Uuid::new_v4().to_string();
            let status = if *quarantined { "skipped" } else { "pending" };
            let completed_at: Option<&str> = quarantined.then_some(now.as_str());
            let changed = conn.execute(
                "INSERT OR IGNORE INTO test_run
                 (id, build_id, test_id, created_at, completed_at, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![run_id, build_id, test_id, now, completed_at, status],
            )?;
            inserted += changed;
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            retry_sqlite(|| conn.execute_batch("COMMIT;"))?;
            Ok(ScheduleStats {
                matched: rows.len(),
                inserted,
            })
        },
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        },
    }
}

fn spawn_async(
    ctx: &AppContext,
    cli: &Cli,
    pattern: &str,
    build: &BuildInfo,
    scheduled: &ScheduleStats,
) -> Result<ExitCode> {
    let short = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();
    let log_path = ctx.runs_dir().join(format!("{}-{short}.log", build.id));
    let stdout_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;
    let stderr_file = stdout_file.try_clone()?;

    let current_exe = env::current_exe()?;
    let mut command = Command::new(current_exe);
    command
        .current_dir(&ctx.repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .env(ASYNC_CHILD_ENV, "1")
        .env(ASYNC_BUILD_ENV, &build.id)
        .env(ASYNC_BINARY_ENV, &build.binary_path)
        .env(ASYNC_PATTERN_ENV, pattern)
        .env(ASYNC_JOBS_ENV, jobs_for(cli, &ctx.config).to_string())
        .env(
            ASYNC_STRATEGY_ENV,
            match strategy_for(cli, &ctx.config) {
                Strategy::Isolated => "isolated",
                Strategy::Batch => "batch",
            },
        )
        .env(
            ASYNC_BATCH_SIZE_ENV,
            batch_size_for(cli, &ctx.config).to_string(),
        );

    command.env("TRIAGE_DB", &ctx.db_path);
    if cli.json {
        command.env(ASYNC_JSON_ENV, "1");
    }
    if let Some(jq) = &cli.jq {
        command.env(ASYNC_JQ_ENV, jq);
    }

    #[cfg(unix)]
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    let child = command.spawn()?;
    let progress = format!("triage status {}", build.id);
    let output = path_for_display(ctx, &log_path);

    if cli.json || cli.jq.is_some() {
        let value = json!({
            "kind": "async_started",
            "build_id": build.id,
            "pid": child.id(),
            "output": output,
            "progress": progress,
            "matched": scheduled.matched,
            "scheduled": scheduled.inserted,
        });
        emit_json_point(&value, cli.jq.as_deref())?;
    } else {
        println!("Build:    {}", build.id);
        println!("Output:   {}", output);
        println!("Progress: {}", progress);
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Clone)]
struct OutputOptions {
    json: bool,
    jq: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct ExecOptions {
    strategy: Strategy,
    batch_size: usize,
}

fn strategy_for(cli: &Cli, config: &Config) -> Strategy {
    cli.strategy
        .or_else(|| {
            env::var("TRIAGE_STRATEGY")
                .ok()
                .and_then(|v| Strategy::parse(&v).ok())
        })
        .unwrap_or(config.strategy)
}

fn batch_size_for(cli: &Cli, config: &Config) -> usize {
    cli.batch_size
        .or_else(|| {
            env::var("TRIAGE_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
        })
        .filter(|n| *n > 0)
        .unwrap_or(config.batch_size.max(1))
}

fn execute_scheduled_run(
    ctx: &AppContext,
    build_id: &str,
    binary_source: &Path,
    pattern: &str,
    jobs: usize,
    output: OutputOptions,
    exec: ExecOptions,
) -> Result<ExitCode> {
    let scratch = ScratchBinary::new(ctx, binary_source)?;
    let started = Instant::now();
    let like = pattern_to_like(pattern);
    let total = count_matching_runs(ctx, build_id, &like)?;

    // With a fixed `batch_size` a small run can starve workers: e.g. 11 tests,
    // 4 jobs, batch_size=16 → one worker takes all 11 and the other three sit
    // idle. Shrink the per-batch limit so every worker gets a few batches to
    // chew through, while still amortizing stdlib init across several tests.
    let effective_batch_size = if exec.strategy == Strategy::Batch && jobs > 0 && total > 0 {
        let target = (total / (jobs * 4)).max(1);
        exec.batch_size.min(target.max(1))
    } else {
        exec.batch_size
    };
    let exec = ExecOptions {
        strategy: exec.strategy,
        batch_size: effective_batch_size,
    };
    let mut sink = JsonLineSink::new(output.json, output.jq.as_deref())?;
    let mut progress = Progress::new(
        !output.json && io::stderr().is_terminal(),
        !output.json,
        total,
    );

    // Seed in-memory counters once; update on each event instead of re-querying
    // the DB after every test. Any drift (e.g. stale reclaims) gets reconciled
    // by the final load_summary below.
    let mut counts = load_summary(ctx, build_id)?.counts;

    sink.emit(json!({
        "kind": "build_started",
        "build_id": build_id,
        "pattern": pattern,
        "test_count": total,
    }))?;
    progress.render(summary_from_counts(&counts), started.elapsed())?;

    let (tx, rx) = mpsc::channel::<WorkerEvent>();
    let mut handles = Vec::new();
    for worker_index in 0..jobs {
        let worker = WorkerConfig {
            db_path: ctx.db_path.clone(),
            logs_dir: ctx.logs_dir(),
            runs_dir: ctx.runs_dir(),
            binary_path: scratch.path.clone(),
            binary_cwd: ctx.binary_cwd(),
            build_id: build_id.to_string(),
            pattern_like: like.clone(),
            harness_prefix: ctx.config.harness_prefix.clone(),
            test_extension: ctx.config.test_extension.clone(),
            stall_threshold: Duration::from_secs(ctx.config.stall_threshold_seconds),
            worker_id: worker_id(worker_index),
            strategy: exec.strategy,
            batch_size: exec.batch_size,
        };
        let tx = tx.clone();
        handles.push(thread::spawn(move || worker_loop(worker, tx)));
    }
    drop(tx);

    for event in rx {
        match &event {
            WorkerEvent::WorkerSpawned { worker_id } => {
                sink.emit(json!({
                    "kind": "worker_spawned",
                    "worker_id": worker_id,
                }))?;
            },
            WorkerEvent::WorkerIdle { worker_id } => {
                sink.emit(json!({
                    "kind": "worker_idle",
                    "worker_id": worker_id,
                }))?;
            },
            WorkerEvent::TestStarted {
                test_run_id,
                test_path,
                worker_id,
            } => {
                shift_count(&mut counts, "pending", "running");
                sink.emit(json!({
                    "kind": "test_started",
                    "test_run_id": test_run_id,
                    "test_path": test_path,
                    "worker_id": worker_id,
                }))?;
            },
            WorkerEvent::TestCompleted {
                test_run_id,
                test_path,
                status,
                exit_code,
                duration_ms,
                failure_message,
            } => {
                shift_count(&mut counts, "running", status);
                let mut value = json!({
                    "kind": "test_completed",
                    "test_run_id": test_run_id,
                    "test_path": test_path,
                    "status": status,
                    "exit_code": exit_code,
                    "duration_ms": duration_ms,
                });
                if let Some(message) = failure_message {
                    value["failure_message"] = json!(message);
                }
                sink.emit(value)?;
                progress.render(summary_from_counts(&counts), started.elapsed())?;
            },
            WorkerEvent::TestRequeued {
                test_run_id,
                test_path,
                worker_id,
            } => {
                // The test had a TestStarted event emitted when the batch was
                // claimed, so move it back out of `running` to keep counts
                // honest. It will get a fresh TestStarted once another
                // worker picks it up.
                shift_count(&mut counts, "running", "pending");
                sink.emit(json!({
                    "kind": "test_requeued",
                    "test_run_id": test_run_id,
                    "test_path": test_path,
                    "worker_id": worker_id,
                }))?;
                progress.render(summary_from_counts(&counts), started.elapsed())?;
            },
            WorkerEvent::Error { worker_id, error } => {
                sink.emit(json!({
                    "kind": "worker_error",
                    "worker_id": worker_id,
                    "error": error,
                }))?;
            },
        }
    }

    for handle in handles {
        match handle.join() {
            Ok(Ok(())) => {},
            Ok(Err(err)) => return Err(err),
            Err(_) => return Err(boxed("worker thread panicked")),
        }
    }

    let summary = load_summary(ctx, build_id)?;
    progress.finish(&summary, started.elapsed())?;
    sink.emit(json!({
        "kind": "build_summary",
        "build_id": build_id,
        "duration_ms": started.elapsed().as_millis() as u64,
        "counts": summary.counts,
        "total": summary.total,
    }))?;
    sink.finish()?;

    if summary.has_failures() {
        Ok(ExitCode::FAILURE)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

#[derive(Debug, Clone)]
struct WorkerConfig {
    db_path: PathBuf,
    logs_dir: PathBuf,
    runs_dir: PathBuf,
    binary_path: PathBuf,
    binary_cwd: PathBuf,
    build_id: String,
    pattern_like: String,
    harness_prefix: String,
    test_extension: String,
    stall_threshold: Duration,
    worker_id: String,
    strategy: Strategy,
    batch_size: usize,
}

#[derive(Debug)]
enum WorkerEvent {
    WorkerSpawned {
        worker_id: String,
    },
    WorkerIdle {
        worker_id: String,
    },
    TestStarted {
        test_run_id: String,
        test_path: String,
        worker_id: String,
    },
    TestCompleted {
        test_run_id: String,
        test_path: String,
        status: String,
        exit_code: Option<i32>,
        duration_ms: u64,
        failure_message: Option<String>,
    },
    /// A test that was claimed (and so already had a `TestStarted` event
    /// emitted) has been returned to the pending queue. Fired by the batch
    /// strategy when a subprocess dies mid-batch: tests that had not yet
    /// begun executing in the subprocess are reset so another worker can
    /// pick them up cleanly.
    TestRequeued {
        test_run_id: String,
        test_path: String,
        worker_id: String,
    },
    Error {
        worker_id: String,
        error: String,
    },
}

fn worker_loop(config: WorkerConfig, tx: mpsc::Sender<WorkerEvent>) -> Result<()> {
    let conn = Connection::open(&config.db_path)?;
    configure_connection(&conn)?;
    send_event(
        &tx,
        WorkerEvent::WorkerSpawned {
            worker_id: config.worker_id.clone(),
        },
    );

    loop {
        reclaim_stale(&conn, &config)?;

        let made_progress = match config.strategy {
            Strategy::Isolated => step_isolated(&conn, &config, &tx)?,
            Strategy::Batch => step_batch(&conn, &config, &tx)?,
        };

        if !made_progress {
            send_event(
                &tx,
                WorkerEvent::WorkerIdle {
                    worker_id: config.worker_id.clone(),
                },
            );
            return Ok(());
        }
    }
}

/// Claim and run one test in its own subprocess. Returns `true` if a test was
/// claimed (and the loop should continue), `false` if the queue is empty.
fn step_isolated(
    conn: &Connection,
    config: &WorkerConfig,
    tx: &mpsc::Sender<WorkerEvent>,
) -> Result<bool> {
    let Some(claim) = claim_next(conn, config)? else {
        return Ok(false);
    };

    send_event(
        tx,
        WorkerEvent::TestStarted {
            test_run_id: claim.run_id.clone(),
            test_path: claim.path.clone(),
            worker_id: config.worker_id.clone(),
        },
    );

    match run_one_test(conn, config, &claim) {
        Ok(result) => {
            send_event(
                tx,
                WorkerEvent::TestCompleted {
                    test_run_id: claim.run_id,
                    test_path: claim.path,
                    status: result.status,
                    exit_code: result.exit_code,
                    duration_ms: result.duration_ms,
                    failure_message: result.failure_message,
                },
            );
            Ok(true)
        },
        Err(err) => {
            send_event(
                tx,
                WorkerEvent::Error {
                    worker_id: config.worker_id.clone(),
                    error: err.to_string(),
                },
            );
            Err(err)
        },
    }
}

/// Claim up to `batch_size` tests and run them in a single subprocess.
fn step_batch(
    conn: &Connection,
    config: &WorkerConfig,
    tx: &mpsc::Sender<WorkerEvent>,
) -> Result<bool> {
    let claims = claim_batch(conn, config)?;
    if claims.is_empty() {
        return Ok(false);
    }

    // Announce every test as started up front so the UI/progress counters
    // reflect work in flight even though only one subprocess is running.
    for claim in &claims {
        send_event(
            tx,
            WorkerEvent::TestStarted {
                test_run_id: claim.run_id.clone(),
                test_path: claim.path.clone(),
                worker_id: config.worker_id.clone(),
            },
        );
    }

    if let Err(err) = run_batch(conn, config, &claims, tx) {
        send_event(
            tx,
            WorkerEvent::Error {
                worker_id: config.worker_id.clone(),
                error: err.to_string(),
            },
        );
        return Err(err);
    }
    Ok(true)
}

fn send_event(tx: &mpsc::Sender<WorkerEvent>, event: WorkerEvent) {
    let _ = tx.send(event);
}

#[derive(Debug)]
struct ClaimedTest {
    run_id: String,
    path: String,
}

fn claim_next(conn: &Connection, config: &WorkerConfig) -> Result<Option<ClaimedTest>> {
    retry_sqlite(|| conn.execute_batch("BEGIN IMMEDIATE;"))?;
    let result = (|| -> Result<Option<ClaimedTest>> {
        let claim = conn
            .query_row(
                "SELECT tr.id, t.path
                 FROM test_run tr
                 JOIN test t ON t.id = tr.test_id
                 WHERE tr.build_id = ?1
                   AND tr.status = 'pending'
                   AND t.removed_at IS NULL
                   AND t.path LIKE ?2 ESCAPE '\\'
                 ORDER BY t.path
                 LIMIT 1",
                params![config.build_id, config.pattern_like],
                |row| {
                    Ok(ClaimedTest {
                        run_id: row.get(0)?,
                        path: row.get(1)?,
                    })
                },
            )
            .optional()?;

        if let Some(claim) = &claim {
            let now = now_string()?;
            let changed = conn.execute(
                "UPDATE test_run
                 SET status = 'running',
                     started_at = ?1,
                     worker_id = ?2,
                     heartbeat_at = ?1
                 WHERE id = ?3 AND status = 'pending'",
                params![now, config.worker_id, claim.run_id],
            )?;
            if changed == 0 {
                return Ok(None);
            }
        }

        Ok(claim)
    })();

    match result {
        Ok(claim) => {
            retry_sqlite(|| conn.execute_batch("COMMIT;"))?;
            Ok(claim)
        },
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        },
    }
}

/// Atomically claim up to `config.batch_size` pending tests for this worker.
///
/// Mirrors the shape of `claim_next` but operates on N rows in one
/// transaction so the full batch is handed to a single `file_tests`
/// invocation (amortizing stdlib init cost across the batch).
fn claim_batch(conn: &Connection, config: &WorkerConfig) -> Result<Vec<ClaimedTest>> {
    if config.batch_size == 0 {
        return Ok(Vec::new());
    }

    retry_sqlite(|| conn.execute_batch("BEGIN IMMEDIATE;"))?;
    let result = (|| -> Result<Vec<ClaimedTest>> {
        let mut stmt = conn.prepare(
            "SELECT tr.id, t.path
             FROM test_run tr
             JOIN test t ON t.id = tr.test_id
             WHERE tr.build_id = ?1
               AND tr.status = 'pending'
               AND t.removed_at IS NULL
               AND t.path LIKE ?2 ESCAPE '\\'
             ORDER BY t.path
             LIMIT ?3",
        )?;
        let candidates: Vec<ClaimedTest> = stmt
            .query_map(
                params![
                    config.build_id,
                    config.pattern_like,
                    config.batch_size as i64
                ],
                |row| {
                    Ok(ClaimedTest {
                        run_id: row.get(0)?,
                        path: row.get(1)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let now = now_string()?;
        let mut claimed = Vec::with_capacity(candidates.len());
        // Individually CAS each row from 'pending' → 'running'. If another
        // worker stole a row between our SELECT and UPDATE, that single row
        // silently drops out — the rest of the batch is still valid.
        for candidate in candidates {
            let changed = conn.execute(
                "UPDATE test_run
                 SET status = 'running',
                     started_at = ?1,
                     worker_id = ?2,
                     heartbeat_at = ?1
                 WHERE id = ?3 AND status = 'pending'",
                params![now, config.worker_id, candidate.run_id],
            )?;
            if changed == 1 {
                claimed.push(candidate);
            }
        }
        Ok(claimed)
    })();

    match result {
        Ok(claims) => {
            retry_sqlite(|| conn.execute_batch("COMMIT;"))?;
            Ok(claims)
        },
        Err(err) => {
            let _ = conn.execute_batch("ROLLBACK;");
            Err(err)
        },
    }
}

fn reclaim_stale(conn: &Connection, config: &WorkerConfig) -> Result<()> {
    let cutoff = timestamp_before(config.stall_threshold)?;
    let now = now_string()?;
    conn.execute(
        "UPDATE test_run
         SET status = 'hung',
             completed_at = ?1,
             failure_message = COALESCE(failure_message, 'worker heartbeat went stale')
         WHERE build_id = ?2
           AND status = 'running'
           AND heartbeat_at IS NOT NULL
           AND heartbeat_at < ?3",
        params![now, config.build_id, cutoff],
    )?;
    Ok(())
}

#[derive(Debug)]
struct TestResult {
    status: String,
    exit_code: Option<i32>,
    duration_ms: u64,
    failure_message: Option<String>,
}

fn run_one_test(
    conn: &Connection,
    config: &WorkerConfig,
    claim: &ClaimedTest,
) -> Result<TestResult> {
    let libtest_name = path_to_libtest(config, &claim.path);
    let started = Instant::now();
    let mut child = Command::new(&config.binary_path)
        .arg("--test-threads=1")
        .arg("--exact")
        .arg(&libtest_name)
        .current_dir(&config.binary_cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let heartbeat_interval = Duration::from_secs(2);
    let mut last_heartbeat = Instant::now();
    let mut timed_out = false;
    // Adaptive backoff: most ks tests finish in a handful of ms, so start with a
    // 1ms poll and grow up to 50ms. Fixed 100ms polls were costing us up to
    // ~100ms of slack per test on the fast path.
    let mut poll = Duration::from_millis(1);
    let max_poll = Duration::from_millis(50);

    loop {
        if child.try_wait()?.is_some() {
            break;
        }

        if started.elapsed() >= config.stall_threshold {
            timed_out = true;
            let _ = child.kill();
            break;
        }

        if last_heartbeat.elapsed() >= heartbeat_interval {
            heartbeat(conn, &claim.run_id)?;
            last_heartbeat = Instant::now();
        }

        thread::sleep(poll);
        poll = (poll * 2).min(max_poll);
    }

    let output = child.wait_with_output()?;
    let duration_ms = started.elapsed().as_millis() as u64;
    let exit_code = exit_code_from_output(&output);
    let parsed = classify_output(timed_out, exit_code, &output, config.stall_threshold);
    // Only write logs for non-pass outcomes. Passing tests are the common case
    // and creating the log dir + two fs::write syscalls per test is measurable
    // when tests are just a few ms each.
    if parsed.status != "passed" {
        write_logs(config, &claim.run_id, &output)?;
    }
    let now = now_string()?;

    let changed = conn.execute(
        "UPDATE test_run
         SET status = ?1,
             completed_at = ?2,
             exit_code = ?3,
             duration_ms = ?4,
             failure_message = ?5,
             heartbeat_at = ?2
         WHERE id = ?6 AND status = 'running'",
        params![
            parsed.status,
            now,
            parsed.exit_code,
            duration_ms as i64,
            parsed.failure_message,
            claim.run_id
        ],
    )?;

    if changed == 0 {
        return Ok(TestResult {
            status: "canceled".to_string(),
            exit_code: None,
            duration_ms,
            failure_message: None,
        });
    }

    Ok(TestResult {
        status: parsed.status,
        exit_code: parsed.exit_code,
        duration_ms,
        failure_message: parsed.failure_message,
    })
}

fn heartbeat(conn: &Connection, test_run_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE test_run
         SET heartbeat_at = ?1
         WHERE id = ?2 AND status = 'running'",
        params![now_string()?, test_run_id],
    )?;
    Ok(())
}

/// Run a batch of claimed tests inside a single `file_tests` subprocess.
///
/// The harness receives test names via `--names-file` and emits JSON events.
/// As per-test `started` / `ok` / `failed` events stream in we record status
/// and failure output immediately, then write logs for non-passing outcomes,
/// update the DB, and emit events.
///
/// Failure handling:
/// * A test the harness reports as `failed` → `failed`, with its `stdout`
///   stored as the log/message.
/// * A test the harness reports as `ok` → `passed`.
/// * If the subprocess exits with an error before completing the batch (e.g.
///   a compiler abort or segfault), the test that was mid-run gets
///   `crashed`, and all tests that had not started yet are requeued for
///   another worker to pick up (`TestRequeued`).
/// * Batch-level timeout works the same way: the active test becomes
///   `timed_out`, the rest are requeued.
fn run_batch(
    conn: &Connection,
    config: &WorkerConfig,
    claims: &[ClaimedTest],
    tx: &mpsc::Sender<WorkerEvent>,
) -> Result<()> {
    let names: Vec<String> = claims
        .iter()
        .map(|c| path_to_libtest(config, &c.path))
        .collect();
    // Reverse-map so we can go from a libtest name (as echoed by the harness)
    // back to its ClaimedTest index.
    let name_to_index: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();

    let names_path = write_names_file(config, &names)?;
    let _names_guard = TempFileGuard(names_path.clone());

    let started = Instant::now();
    // Allow roughly per-test stall budget, scaled by the batch size, with a
    // sensible floor so small batches don't starve on a slow first test.
    let batch_timeout = config
        .stall_threshold
        .saturating_mul(claims.len().max(4) as u32);

    let mut child = Command::new(&config.binary_path)
        .arg("--test-threads=1")
        .arg("--format")
        .arg("json")
        .arg("--names-file")
        .arg(&names_path)
        .current_dir(&config.binary_cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Drain stdout on a background thread as lines arrive. This is what gives
    // us live heartbeats: we know which test is active by watching the JSON
    // `started` event that precedes its result.
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let (line_tx, line_rx) = mpsc::channel::<String>();
    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        let mut raw = Vec::new();
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    raw.push(l.clone());
                    if line_tx.send(l).is_err() {
                        break;
                    }
                },
                Err(_) => break,
            }
        }
        raw
    });
    let stderr_thread = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf = String::new();
        let _ = std::io::Read::read_to_string(&mut reader, &mut buf);
        buf
    });

    let mut parser = BatchParser::new(&names);
    let heartbeat_interval = Duration::from_secs(2);
    let mut last_heartbeat = Instant::now();
    let mut poll = Duration::from_millis(1);
    let max_poll = Duration::from_millis(50);
    let mut timed_out = false;

    loop {
        // Drain any stdout lines currently available without blocking.
        let mut drained_any = false;
        loop {
            match line_rx.try_recv() {
                Ok(line) => {
                    drained_any = true;
                    parser.observe(&line);
                },
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }

        if let Some(_status) = child.try_wait()? {
            break;
        }

        if started.elapsed() >= batch_timeout {
            timed_out = true;
            let _ = child.kill();
            break;
        }

        if last_heartbeat.elapsed() >= heartbeat_interval {
            let now = now_string()?;
            for (i, claim) in claims.iter().enumerate() {
                if parser.status_of(i).is_none() {
                    let _ = conn.execute(
                        "UPDATE test_run
                         SET heartbeat_at = ?1
                         WHERE id = ?2 AND status = 'running'",
                        params![now, claim.run_id],
                    )?;
                }
            }
            last_heartbeat = Instant::now();
        }

        if drained_any {
            poll = Duration::from_millis(1);
        } else {
            thread::sleep(poll);
            poll = (poll * 2).min(max_poll);
        }
    }

    // Drain any remaining stdout lines, then reap the child and join readers.
    while let Ok(line) = line_rx.recv() {
        parser.observe(&line);
    }
    let exit_status = child.wait()?;
    let raw_stdout = stdout_thread.join().unwrap_or_default();
    let stderr_text = stderr_thread.join().unwrap_or_default();
    let full_stdout = raw_stdout.join("\n");
    parser.finalize(&full_stdout);

    let duration_ms = started.elapsed().as_millis() as u64;
    let exit_code = exit_status.code();
    let clean_exit = exit_status.success();

    // Now materialize per-test outcomes. For any test the harness did report,
    // we use that status verbatim. For tests with no report, we decide based
    // on how the process ended.
    for (i, claim) in claims.iter().enumerate() {
        let libtest_name = &names[i];
        let observed = parser.status_of(i);
        let (status, failure_message, per_test_log) = match observed {
            Some(BatchTestStatus::Passed) => ("passed", None, None),
            Some(BatchTestStatus::Failed) => {
                let message = parser.failure_message(libtest_name);
                ("failed", Some(message.clone()), Some(message))
            },
            Some(BatchTestStatus::Ignored) => ("ignored", None, None),
            None => {
                // No result line for this test.
                if timed_out && parser.active_index() == Some(i) {
                    (
                        "timed_out",
                        Some(format!(
                            "batch exceeded {} second timeout while this test was running",
                            batch_timeout.as_secs()
                        )),
                        Some(full_stdout.clone()),
                    )
                } else if !clean_exit && parser.active_index() == Some(i) {
                    let reason = match exit_code {
                        Some(code) if code < 0 => {
                            format!("batch subprocess exited with signal {}", -code)
                        },
                        Some(code) => format!("batch subprocess exited with code {code}"),
                        None => "batch subprocess did not exit cleanly".to_string(),
                    };
                    ("crashed", Some(reason), Some(full_stdout.clone()))
                } else if !clean_exit {
                    // Subprocess died, and this test was queued behind the
                    // active one. Requeue it so another worker picks it up;
                    // attributing "crashed" would be misleading.
                    let _ = conn.execute(
                        "UPDATE test_run
                         SET status = 'pending',
                             started_at = NULL,
                             heartbeat_at = NULL,
                             worker_id = NULL
                         WHERE id = ?1 AND status = 'running'",
                        params![claim.run_id],
                    )?;
                    send_event(
                        tx,
                        WorkerEvent::TestRequeued {
                            test_run_id: claim.run_id.clone(),
                            test_path: claim.path.clone(),
                            worker_id: config.worker_id.clone(),
                        },
                    );
                    continue;
                } else {
                    // Clean exit but the harness never mentioned this test.
                    // That would mean `--names-file` filtered it out, which
                    // is a bug on our side. Treat as crashed so it surfaces.
                    (
                        "crashed",
                        Some(format!(
                            "subprocess exited cleanly but did not report a result for {libtest_name}"
                        )),
                        Some(full_stdout.clone()),
                    )
                }
            },
        };

        // Write logs only for non-passing outcomes (match Phase A).
        if status != "passed" {
            write_batch_logs(config, &claim.run_id, per_test_log.as_deref(), &stderr_text)?;
        }

        let now = now_string()?;
        let changed = conn.execute(
            "UPDATE test_run
             SET status = ?1,
                 completed_at = ?2,
                 exit_code = ?3,
                 duration_ms = ?4,
                 failure_message = ?5,
                 heartbeat_at = ?2
             WHERE id = ?6 AND status = 'running'",
            params![
                status,
                now,
                exit_code,
                duration_ms as i64,
                failure_message,
                claim.run_id
            ],
        )?;

        if changed == 0 {
            // Another process flipped the row (e.g. `cancel`). Nothing more
            // to do — just emit the event so listeners stay consistent.
            send_event(
                tx,
                WorkerEvent::TestCompleted {
                    test_run_id: claim.run_id.clone(),
                    test_path: claim.path.clone(),
                    status: "canceled".to_string(),
                    exit_code: None,
                    duration_ms,
                    failure_message: None,
                },
            );
            continue;
        }

        send_event(
            tx,
            WorkerEvent::TestCompleted {
                test_run_id: claim.run_id.clone(),
                test_path: claim.path.clone(),
                status: status.to_string(),
                exit_code,
                duration_ms,
                failure_message,
            },
        );
    }

    // Silence unused variable warning for name_to_index — it is held here so
    // future enhancements (e.g. looking up a claim from a streamed failure
    // block) have the mapping ready; the current observe-by-sequence path
    // doesn't require it.
    let _ = name_to_index;
    Ok(())
}

fn write_names_file(config: &WorkerConfig, names: &[String]) -> Result<PathBuf> {
    fs::create_dir_all(&config.runs_dir)?;
    let unique = format!(
        "{}-{}-{}.names",
        config.worker_id,
        std::process::id(),
        now_nanos(),
    );
    let path = config.runs_dir.join(unique);
    let mut buf = String::with_capacity(names.iter().map(|n| n.len() + 1).sum());
    for name in names {
        buf.push_str(name);
        buf.push('\n');
    }
    fs::write(&path, buf)?;
    Ok(path)
}

struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn write_batch_logs(
    config: &WorkerConfig,
    run_id: &str,
    stdout: Option<&str>,
    stderr: &str,
) -> Result<()> {
    let dir = config.logs_dir.join(run_id);
    fs::create_dir_all(&dir)?;
    if let Some(stdout) = stdout {
        if !stdout.is_empty() {
            fs::write(dir.join("stdout"), stdout)?;
        }
    }
    if !stderr.is_empty() {
        fs::write(dir.join("stderr"), stderr)?;
    }
    Ok(())
}

/// Streaming/finalizing parser for libtest-mimic output.
///
/// Batch mode asks the harness for JSON output and consumes events like:
///   `{ "type": "test", "event": "started", "name": "run_ks_test::foo.ks" }`
///   `{ "type": "test", "event": "ok", "name": "run_ks_test::foo.ks" }`
///   `{ "type": "test", "event": "failed", "name": "...", "stdout": "..." }`
///
/// We also keep support for the older pretty output shape:
///   test run_ks_test::foo.ks ... ok
///   test run_ks_test::bar.ks ... FAILED
///   test run_ks_test::baz.ks ... ignored
///
/// Capturing `started` as it streams in keeps `active_index` accurate for
/// timeout/crash attribution.
#[derive(Debug)]
struct BatchParser {
    name_to_index: HashMap<String, usize>,
    results: Vec<Option<BatchTestStatus>>,
    active: Option<usize>,
    failure_blocks: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchTestStatus {
    Passed,
    Failed,
    Ignored,
}

impl BatchParser {
    fn new(names: &[String]) -> Self {
        Self {
            name_to_index: names
                .iter()
                .enumerate()
                .map(|(i, n)| (n.clone(), i))
                .collect(),
            results: vec![None; names.len()],
            active: None,
            failure_blocks: HashMap::new(),
        }
    }

    fn observe(&mut self, line: &str) {
        if self.observe_json(line) {
            return;
        }

        let Some(rest) = line.strip_prefix("test ") else {
            return;
        };
        // Two shapes:
        //   `test NAME ... STATUS`         — one-line result
        //   `test NAME ...`                — start (harness has not yet
        //                                    printed the result; happens when
        //                                    output is line-buffered and the
        //                                    test is mid-run)
        let Some(sep) = rest.find(" ... ") else {
            // Bare name (no " ... "). Treat as active-test announcement.
            let name = rest.trim();
            if let Some(&idx) = self.name_to_index.get(name) {
                self.active = Some(idx);
            }
            return;
        };
        let name = rest[..sep].trim();
        let after = rest[sep + " ... ".len()..].trim();
        let Some(&idx) = self.name_to_index.get(name) else {
            return;
        };

        if after.is_empty() {
            // Harness printed the test start but result hasn't flushed yet.
            self.active = Some(idx);
            return;
        }

        let status = match after {
            "ok" => BatchTestStatus::Passed,
            "FAILED" => BatchTestStatus::Failed,
            s if s.starts_with("ignored") => BatchTestStatus::Ignored,
            _ => return,
        };
        self.results[idx] = Some(status);
        if self.active == Some(idx) {
            self.active = None;
        }
    }

    fn observe_json(&mut self, line: &str) -> bool {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return false;
        };
        if value.get("type").and_then(Value::as_str) != Some("test") {
            return true;
        }

        let Some(name) = value.get("name").and_then(Value::as_str) else {
            return true;
        };
        let Some(&idx) = self.name_to_index.get(name) else {
            return true;
        };
        let Some(event) = value.get("event").and_then(Value::as_str) else {
            return true;
        };

        match event {
            "started" => {
                self.active = Some(idx);
            },
            "ok" => {
                self.results[idx] = Some(BatchTestStatus::Passed);
                if self.active == Some(idx) {
                    self.active = None;
                }
            },
            "failed" => {
                self.results[idx] = Some(BatchTestStatus::Failed);
                if let Some(stdout) = value.get("stdout").and_then(Value::as_str) {
                    self.failure_blocks
                        .insert(name.to_string(), stdout.trim_end().to_string());
                }
                if self.active == Some(idx) {
                    self.active = None;
                }
            },
            "ignored" => {
                self.results[idx] = Some(BatchTestStatus::Ignored);
                if self.active == Some(idx) {
                    self.active = None;
                }
            },
            _ => {},
        }

        true
    }

    fn finalize(&mut self, full_stdout: &str) {
        // Parse `---- NAME stdout ----` blocks from the failures section.
        // Lines between that header and the next `----` header (or the
        // `failures:` summary) are the per-test failure output.
        let mut current: Option<(String, String)> = None;
        for line in full_stdout.lines() {
            if let Some(rest) = line.strip_prefix("---- ") {
                if let Some(name) = rest.strip_suffix(" stdout ----") {
                    if let Some((n, buf)) = current.take() {
                        self.failure_blocks.insert(n, buf.trim_end().to_string());
                    }
                    if self.name_to_index.contains_key(name) {
                        current = Some((name.to_string(), String::new()));
                    } else {
                        current = None;
                    }
                    continue;
                }
            }
            if line.starts_with("failures:") && current.is_some() {
                if let Some((n, buf)) = current.take() {
                    self.failure_blocks.insert(n, buf.trim_end().to_string());
                }
                continue;
            }
            if line.starts_with("test result:") && current.is_some() {
                if let Some((n, buf)) = current.take() {
                    self.failure_blocks.insert(n, buf.trim_end().to_string());
                }
                continue;
            }
            if let Some((_, buf)) = current.as_mut() {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        if let Some((n, buf)) = current.take() {
            self.failure_blocks.insert(n, buf.trim_end().to_string());
        }
    }

    fn status_of(&self, index: usize) -> Option<BatchTestStatus> {
        self.results.get(index).copied().flatten()
    }

    fn active_index(&self) -> Option<usize> {
        self.active
    }

    fn failure_message(&self, libtest_name: &str) -> String {
        self.failure_blocks
            .get(libtest_name)
            .cloned()
            .unwrap_or_else(|| "test failed (no failure output captured)".to_string())
    }
}

fn write_logs(config: &WorkerConfig, run_id: &str, output: &Output) -> Result<()> {
    let dir = config.logs_dir.join(run_id);
    fs::create_dir_all(&dir)?;
    if !output.stdout.is_empty() {
        fs::write(dir.join("stdout"), &output.stdout)?;
    }
    if !output.stderr.is_empty() {
        fs::write(dir.join("stderr"), &output.stderr)?;
    }
    Ok(())
}

#[derive(Debug)]
struct ParsedResult {
    status: String,
    exit_code: Option<i32>,
    failure_message: Option<String>,
}

fn classify_output(
    timed_out: bool,
    exit_code: Option<i32>,
    output: &Output,
    timeout: Duration,
) -> ParsedResult {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    if timed_out {
        return ParsedResult {
            status: "timed_out".to_string(),
            exit_code,
            failure_message: Some(format!(
                "test exceeded {} second timeout",
                timeout.as_secs()
            )),
        };
    }

    if stdout.lines().any(|line| line.contains(" ... ok")) && output.status.success() {
        return ParsedResult {
            status: "passed".to_string(),
            exit_code,
            failure_message: None,
        };
    }

    if stdout.lines().any(|line| line.contains(" ... FAILED")) {
        return ParsedResult {
            status: "failed".to_string(),
            exit_code,
            failure_message: extract_failure_message(&combined),
        };
    }

    if combined.contains("panicked at") || combined.contains("panicked") {
        return ParsedResult {
            status: "panicked".to_string(),
            exit_code,
            failure_message: Some(non_empty_message(&combined, "test panicked")),
        };
    }

    if !output.status.success() {
        let reason = match exit_code {
            Some(code) if code < 0 => format!("no libtest output; exited with signal {}", -code),
            Some(code) => format!("no libtest output; exit code {code}"),
            None => "no libtest output; process did not exit cleanly".to_string(),
        };
        return ParsedResult {
            status: "crashed".to_string(),
            exit_code,
            failure_message: Some(reason),
        };
    }

    ParsedResult {
        status: "passed".to_string(),
        exit_code,
        failure_message: None,
    }
}

fn extract_failure_message(output: &str) -> Option<String> {
    if let Some(start) = output.find("failures:") {
        let rest = &output[start..];
        let end = rest.find("\ntest result:").unwrap_or(rest.len());
        return Some(rest[..end].trim().to_string());
    }
    Some(non_empty_message(output, "test failed"))
}

fn non_empty_message(output: &str, fallback: &str) -> String {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn status_command(ctx: &AppContext, cli: &Cli, build_id: Option<&str>) -> Result<ExitCode> {
    let conn = ctx.open_db()?;
    let build_id = match build_id {
        Some(id) => id.to_string(),
        None => latest_build_id(&conn)?.ok_or_else(|| boxed("no builds recorded yet"))?,
    };
    let build = load_build(&conn, &build_id)?.ok_or_else(|| boxed("build not found"))?;
    let summary = load_summary(ctx, &build_id)?;
    let failures = if cli.show_failures {
        load_failure_rows(&conn, &build_id, cli.show_messages)?
    } else {
        Vec::new()
    };

    let mut value = json!({
        "kind": "status",
        "build": build,
        "counts": summary.counts,
        "total": summary.total,
    });
    if cli.show_failures {
        value["failures"] = json!(&failures);
    }

    emit_point(cli, value, || {
        println!("Build:   {}", build_id);
        println!("Commit:  {}", build.commit_sha);
        if let Some(branch) = &build.branch {
            println!("Branch:  {branch}");
        }
        println!("Dirty:   {}", build.dirty);
        println!("Created: {}", build.created_at);
        print_counts(&summary);
        if cli.show_failures {
            print_failure_rows(&failures, cli.show_messages);
        }
        Ok(())
    })?;
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Clone, Serialize)]
struct FailureRow {
    test_run_id: String,
    test_path: String,
    status: String,
    exit_code: Option<i64>,
    duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_message: Option<String>,
}

fn load_failure_rows(
    conn: &Connection,
    build_id: &str,
    include_messages: bool,
) -> Result<Vec<FailureRow>> {
    let mut stmt = conn.prepare(
        "SELECT tr.id, t.path, tr.status, tr.exit_code, tr.duration_ms, tr.failure_message
         FROM test_run tr
         JOIN test t ON t.id = tr.test_id
         WHERE tr.build_id = ?1
           AND tr.status IN ('failed', 'timed_out', 'hung', 'crashed', 'panicked', 'canceled')
         ORDER BY t.path",
    )?;
    let rows = stmt.query_map(params![build_id], |row| {
        let failure_message = if include_messages { row.get(5)? } else { None };
        Ok(FailureRow {
            test_run_id: row.get(0)?,
            test_path: row.get(1)?,
            status: row.get(2)?,
            exit_code: row.get(3)?,
            duration_ms: row.get(4)?,
            failure_message,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn print_failure_rows(failures: &[FailureRow], show_messages: bool) {
    if failures.is_empty() {
        println!("Failures: none");
        return;
    }

    println!("Failures:");
    for failure in failures {
        match failure.duration_ms {
            Some(duration_ms) => println!(
                "  {}  {}  {}ms",
                failure.status, failure.test_path, duration_ms
            ),
            None => println!("  {}  {}", failure.status, failure.test_path),
        }

        if show_messages {
            if let Some(message) = &failure.failure_message {
                print_indented_message(message);
            }
        }
    }
}

fn print_indented_message(message: &str) {
    for line in message.lines() {
        println!("    {line}");
    }
}

fn builds_command(ctx: &AppContext, cli: &Cli) -> Result<ExitCode> {
    let conn = ctx.open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, binary_hash, commit_sha, branch, dirty, created_at
         FROM build
         ORDER BY created_at DESC
         LIMIT 20",
    )?;
    let rows = stmt.query_map([], build_from_row)?;
    let builds = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut values = Vec::new();
    for build in &builds {
        let summary = load_summary(ctx, &build.id)?;
        values.push(json!({
            "build": build,
            "counts": summary.counts,
            "total": summary.total,
        }));
    }
    let value = json!({
        "kind": "builds",
        "builds": values,
    });

    emit_point(cli, value, || {
        for build in &builds {
            let summary = load_summary(ctx, &build.id)?;
            println!(
                "{}  {}  {}{}",
                &build.id,
                build.created_at,
                short_sha(&build.commit_sha),
                if build.dirty { " dirty" } else { "" }
            );
            print!("  ");
            print_counts_inline(&summary);
            println!();
        }
        Ok(())
    })?;
    Ok(ExitCode::SUCCESS)
}

fn history_command(ctx: &AppContext, cli: &Cli, test: &str) -> Result<ExitCode> {
    let conn = ctx.open_db()?;
    let test_id: String = conn
        .query_row(
            "SELECT id FROM test WHERE path = ?1",
            params![test],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| boxed(format!("test `{test}` has not been discovered")))?;

    let mut stmt = conn.prepare(
        "SELECT tr.id, tr.build_id, tr.created_at, tr.started_at, tr.completed_at,
                tr.status, tr.exit_code, tr.duration_ms, tr.failure_message,
                b.commit_sha, b.branch, b.dirty
         FROM test_run tr
         JOIN build b ON b.id = tr.build_id
         WHERE tr.test_id = ?1
         ORDER BY tr.created_at DESC
         LIMIT 50",
    )?;
    let rows = stmt.query_map(params![test_id], |row| {
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "build_id": row.get::<_, String>(1)?,
            "created_at": row.get::<_, String>(2)?,
            "started_at": row.get::<_, Option<String>>(3)?,
            "completed_at": row.get::<_, Option<String>>(4)?,
            "status": row.get::<_, String>(5)?,
            "exit_code": row.get::<_, Option<i64>>(6)?,
            "duration_ms": row.get::<_, Option<i64>>(7)?,
            "failure_message": row.get::<_, Option<String>>(8)?,
            "commit_sha": row.get::<_, String>(9)?,
            "branch": row.get::<_, Option<String>>(10)?,
            "dirty": row.get::<_, i64>(11)? != 0,
        }))
    })?;
    let runs = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let value = json!({
        "kind": "history",
        "test": test,
        "runs": runs,
    });

    emit_point(cli, value, || {
        println!("Test: {test}");
        for run in &runs {
            println!(
                "{}  {}  {}  {}ms",
                run["build_id"].as_str().unwrap_or(""),
                run["created_at"].as_str().unwrap_or(""),
                run["status"].as_str().unwrap_or(""),
                run["duration_ms"].as_i64().unwrap_or(0)
            );
        }
        Ok(())
    })?;
    Ok(ExitCode::SUCCESS)
}

fn quarantine_command(ctx: &AppContext, cli: &Cli, test: &str, reason: &str) -> Result<ExitCode> {
    let conn = ctx.open_db()?;
    let changed = conn.execute(
        "UPDATE test
         SET quarantined = 1, skip_reason = ?1
         WHERE path = ?2 AND removed_at IS NULL",
        params![reason, test],
    )?;
    if changed == 0 {
        return Err(boxed(format!(
            "test `{test}` has not been discovered or has been removed"
        )));
    }
    let value = json!({
        "kind": "quarantine",
        "test": test,
        "reason": reason,
        "changed": changed,
    });
    emit_point(cli, value, || {
        println!("Quarantined {test}: {reason}");
        Ok(())
    })?;
    Ok(ExitCode::SUCCESS)
}

fn unquarantine_command(ctx: &AppContext, cli: &Cli, test: &str) -> Result<ExitCode> {
    let conn = ctx.open_db()?;
    let changed = conn.execute(
        "UPDATE test
         SET quarantined = 0, skip_reason = NULL
         WHERE path = ?1 AND removed_at IS NULL",
        params![test],
    )?;
    if changed == 0 {
        return Err(boxed(format!(
            "test `{test}` has not been discovered or has been removed"
        )));
    }
    let value = json!({
        "kind": "unquarantine",
        "test": test,
        "changed": changed,
    });
    emit_point(cli, value, || {
        println!("Unquarantined {test}");
        Ok(())
    })?;
    Ok(ExitCode::SUCCESS)
}

fn cancel_command(ctx: &AppContext, cli: &Cli, build_id: &str) -> Result<ExitCode> {
    let conn = ctx.open_db()?;
    let now = now_string()?;
    let changed = conn.execute(
        "UPDATE test_run
         SET status = 'canceled', completed_at = ?1
         WHERE build_id = ?2 AND status IN ('pending', 'running')",
        params![now, build_id],
    )?;
    let value = json!({
        "kind": "cancel",
        "build_id": build_id,
        "changed": changed,
    });
    emit_point(cli, value, || {
        println!("Canceled {changed} pending/running rows for {build_id}");
        Ok(())
    })?;
    Ok(ExitCode::SUCCESS)
}

#[derive(Debug, Clone, Serialize)]
struct BuildRow {
    id: String,
    binary_hash: String,
    commit_sha: String,
    branch: Option<String>,
    dirty: bool,
    created_at: String,
}

fn latest_build_id(conn: &Connection) -> Result<Option<String>> {
    conn.query_row(
        "SELECT id FROM build ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn load_build(conn: &Connection, build_id: &str) -> Result<Option<BuildRow>> {
    conn.query_row(
        "SELECT id, binary_hash, commit_sha, branch, dirty, created_at
         FROM build
         WHERE id = ?1",
        params![build_id],
        build_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn build_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BuildRow> {
    Ok(BuildRow {
        id: row.get(0)?,
        binary_hash: row.get(1)?,
        commit_sha: row.get(2)?,
        branch: row.get(3)?,
        dirty: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
    })
}

#[derive(Debug, Clone, Serialize)]
struct Summary {
    counts: BTreeMap<String, usize>,
    total: usize,
}

impl Summary {
    fn has_failures(&self) -> bool {
        [
            "failed",
            "timed_out",
            "hung",
            "crashed",
            "panicked",
            "canceled",
        ]
        .iter()
        .any(|status| self.count(status) > 0)
    }

    fn count(&self, status: &str) -> usize {
        self.counts.get(status).copied().unwrap_or(0)
    }
}

fn summary_from_counts(counts: &BTreeMap<String, usize>) -> Summary {
    Summary {
        counts: counts.clone(),
        total: counts.values().sum(),
    }
}

fn shift_count(counts: &mut BTreeMap<String, usize>, from: &str, to: &str) {
    if let Some(value) = counts.get_mut(from) {
        *value = value.saturating_sub(1);
    }
    *counts.entry(to.to_string()).or_insert(0) += 1;
}

fn load_summary(ctx: &AppContext, build_id: &str) -> Result<Summary> {
    let conn = ctx.open_db()?;
    let mut counts = BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT status, COUNT(*)
         FROM test_run
         WHERE build_id = ?1
         GROUP BY status
         ORDER BY status",
    )?;
    let rows = stmt.query_map(params![build_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
    })?;
    for row in rows {
        let (status, count) = row?;
        counts.insert(status, count);
    }
    let total = counts.values().sum();
    Ok(Summary { counts, total })
}

fn count_matching_runs(ctx: &AppContext, build_id: &str, like: &str) -> Result<usize> {
    let conn = ctx.open_db()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*)
         FROM test_run tr
         JOIN test t ON t.id = tr.test_id
         WHERE tr.build_id = ?1
           AND t.path LIKE ?2 ESCAPE '\\'",
        params![build_id, like],
        |row| row.get(0),
    )?;
    Ok(count as usize)
}

fn print_counts(summary: &Summary) {
    print!("Counts:  ");
    print_counts_inline(summary);
    println!();
}

fn print_counts_inline(summary: &Summary) {
    let statuses = [
        "passed",
        "failed",
        "skipped",
        "timed_out",
        "hung",
        "crashed",
        "panicked",
        "canceled",
        "running",
        "pending",
    ];
    let mut first = true;
    for status in statuses {
        let count = summary.count(status);
        if count == 0 {
            continue;
        }
        if !first {
            print!(", ");
        }
        first = false;
        print!("{count} {status}");
    }
    if first {
        print!("0 tests");
    }
}

#[derive(Debug)]
struct Progress {
    enabled: bool,
    text_summary: bool,
    total: usize,
}

impl Progress {
    fn new(enabled: bool, text_summary: bool, total: usize) -> Self {
        Self {
            enabled,
            text_summary,
            total,
        }
    }

    fn render(&mut self, summary: Summary, elapsed: Duration) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let running = summary.count("running");
        let pending = summary.count("pending");
        let done = self.total.saturating_sub(running + pending);
        let failed = summary.count("failed")
            + summary.count("timed_out")
            + summary.count("hung")
            + summary.count("crashed")
            + summary.count("panicked");
        let width = 24usize;
        let filled = if self.total == 0 {
            width
        } else {
            width * done / self.total
        };
        let bar = format!("{}{}", "=".repeat(filled), " ".repeat(width - filled));
        eprint!(
            "\r[{bar}] {done}/{} . {failed} failed . {}s",
            self.total,
            elapsed.as_secs()
        );
        io::stderr().flush()?;
        Ok(())
    }

    fn finish(&mut self, summary: &Summary, elapsed: Duration) -> Result<()> {
        if self.enabled {
            self.render(summary.clone(), elapsed)?;
            eprintln!();
        } else if self.text_summary {
            print_counts(summary);
        }
        Ok(())
    }
}

struct JsonLineSink {
    json: bool,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

impl JsonLineSink {
    fn new(json: bool, jq: Option<&str>) -> Result<Self> {
        if let Some(expr) = jq {
            let mut child = Command::new("jq")
                .arg("-c")
                .arg(expr)
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|err| boxed(format!("failed to run jq: {err}")))?;
            let stdin = child
                .stdin
                .take()
                .ok_or_else(|| boxed("failed to open jq stdin"))?;
            return Ok(Self {
                json: true,
                child: Some(child),
                stdin: Some(stdin),
            });
        }

        Ok(Self {
            json,
            child: None,
            stdin: None,
        })
    }

    fn emit(&mut self, value: Value) -> Result<()> {
        if !self.json {
            return Ok(());
        }
        let line = serde_json::to_string(&value)?;
        if let Some(stdin) = &mut self.stdin {
            writeln!(stdin, "{line}")?;
        } else {
            println!("{line}");
        }
        Ok(())
    }

    fn finish(mut self) -> Result<()> {
        drop(self.stdin.take());
        if let Some(mut child) = self.child.take() {
            let status = child.wait()?;
            if !status.success() {
                return Err(boxed(format!("jq exited with {status}")));
            }
        }
        Ok(())
    }
}

fn emit_point<F>(cli: &Cli, value: Value, text: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    if cli.json || cli.jq.is_some() {
        emit_json_point(&value, cli.jq.as_deref())
    } else {
        text()
    }
}

fn emit_json_point(value: &Value, jq: Option<&str>) -> Result<()> {
    if let Some(expr) = jq {
        let mut child = Command::new("jq")
            .arg("-c")
            .arg(expr)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|err| boxed(format!("failed to run jq: {err}")))?;
        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| boxed("failed to open jq stdin"))?;
            serde_json::to_writer(&mut *stdin, value)?;
            writeln!(stdin)?;
        }
        let status = child.wait()?;
        if !status.success() {
            return Err(boxed(format!("jq exited with {status}")));
        }
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

struct ScratchBinary {
    dir: PathBuf,
    path: PathBuf,
}

impl ScratchBinary {
    fn new(ctx: &AppContext, source: &Path) -> Result<Self> {
        let invocation_id = Uuid::new_v4().to_string();
        let dir = ctx.binaries_dir().join(invocation_id);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("pid"), std::process::id().to_string())?;
        let path = dir.join("file_tests");
        fs::copy(source, &path)?;
        #[cfg(unix)]
        {
            let mode = fs::metadata(source)?.permissions().mode();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode | 0o700))?;
        }
        Ok(Self { dir, path })
    }
}

impl Drop for ScratchBinary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[derive(Debug)]
struct GitInfo {
    commit_sha: String,
    branch: Option<String>,
    dirty: bool,
}

fn git_info(repo_root: &Path) -> GitInfo {
    let commit_sha = command_stdout(repo_root, "git", &["rev-parse", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    let branch = command_stdout(repo_root, "git", &["branch", "--show-current"])
        .filter(|branch| !branch.is_empty());
    let dirty = command_stdout(repo_root, "git", &["status", "--porcelain"])
        .is_some_and(|status| !status.trim().is_empty());
    GitInfo {
        commit_sha,
        branch,
        dirty,
    }
}

fn command_stdout(cwd: &Path, program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn jobs_for(cli: &Cli, config: &Config) -> usize {
    cli.jobs
        .or_else(|| {
            env::var("TRIAGE_JOBS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
        })
        .filter(|jobs| *jobs > 0)
        .unwrap_or(config.jobs.max(1))
}

fn path_to_libtest(config: &WorkerConfig, path: &str) -> String {
    format!(
        "{}{}{}",
        config.harness_prefix,
        path.replace('.', "/"),
        config.test_extension
    )
}

fn libtest_to_path(config: &Config, name: &str) -> Option<String> {
    let without_prefix = name.strip_prefix(&config.harness_prefix)?;
    let without_ext = without_prefix.strip_suffix(&config.test_extension)?;
    Some(without_ext.replace('/', "."))
}

fn pattern_to_like(pattern: &str) -> String {
    let mut out = String::new();
    for ch in pattern.chars() {
        match ch {
            '*' => out.push('%'),
            '%' | '_' | '\\' => {
                out.push('\\');
                out.push(ch);
            },
            _ => out.push(ch),
        }
    }
    out
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn now_string() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn timestamp_before(duration: Duration) -> Result<String> {
    let seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
    Ok((OffsetDateTime::now_utc() - TimeDuration::seconds(seconds)).format(&Rfc3339)?)
}

fn worker_id(index: usize) -> String {
    format!(
        "{}:{}:{}:{}",
        hostname(),
        std::process::id(),
        index,
        Uuid::new_v4()
    )
}

fn hostname() -> String {
    env::var("HOSTNAME")
        .or_else(|_| env::var("COMPUTERNAME"))
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown-host".to_string())
}

fn exit_code_from_output(output: &Output) -> Option<i32> {
    if let Some(code) = output.status.code() {
        return Some(code);
    }
    #[cfg(unix)]
    {
        return output.status.signal().map(|signal| -signal);
    }
    #[allow(unreachable_code)]
    None
}

fn exit_code(code: i32) -> ExitCode {
    ExitCode::from(code.clamp(1, 255) as u8)
}

fn is_executable(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        let mode = fs::metadata(path)?.permissions().mode();
        Ok(mode & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(path.is_file())
    }
}

fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        if libc::kill(pid as libc::pid_t, 0) == 0 {
            return true;
        }
        let Some(errno) = io::Error::last_os_error().raw_os_error() else {
            return true;
        };
        errno != libc::ESRCH
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn path_for_display(ctx: &AppContext, path: &Path) -> String {
    path.strip_prefix(&ctx.repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(12).collect()
}

fn boxed(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    io::Error::other(message.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libtest_names_round_trip() {
        let config = Config::default();
        let path = libtest_to_path(
            &config,
            "run_ks_test::attributes/declarations/case_with_attribute.ks",
        )
        .unwrap();
        assert_eq!(path, "attributes.declarations.case_with_attribute");

        let worker = WorkerConfig {
            db_path: PathBuf::new(),
            logs_dir: PathBuf::new(),
            runs_dir: PathBuf::new(),
            binary_path: PathBuf::new(),
            binary_cwd: PathBuf::new(),
            build_id: String::new(),
            pattern_like: String::new(),
            harness_prefix: config.harness_prefix,
            test_extension: config.test_extension,
            stall_threshold: Duration::from_secs(30),
            worker_id: String::new(),
            strategy: Strategy::Batch,
            batch_size: 16,
        };
        assert_eq!(
            path_to_libtest(&worker, &path),
            "run_ks_test::attributes/declarations/case_with_attribute.ks"
        );
    }

    #[test]
    fn pattern_escapes_sql_like_wildcards() {
        assert_eq!(pattern_to_like("declarations.*"), "declarations.%");
        assert_eq!(pattern_to_like("a_b%"), "a\\_b\\%");
    }

    #[test]
    fn batch_parser_attributes_pretty_output() {
        let names = vec![
            "run_ks_test::foo/ok.ks".to_string(),
            "run_ks_test::foo/fail.ks".to_string(),
            "run_ks_test::foo/skip.ks".to_string(),
        ];
        let mut parser = BatchParser::new(&names);

        for line in [
            "running 3 tests",
            "test run_ks_test::foo/ok.ks ... ok",
            "test run_ks_test::foo/fail.ks ... FAILED",
            "test run_ks_test::foo/skip.ks ... ignored",
            "",
            "failures:",
            "",
            "---- run_ks_test::foo/fail.ks stdout ----",
            "assertion failed: values differ",
            "thread 'main' panicked at ...",
            "",
            "",
            "failures:",
            "    run_ks_test::foo/fail.ks",
            "",
            "test result: FAILED. 1 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out",
        ] {
            parser.observe(line);
        }
        parser.finalize(
            &[
                "",
                "failures:",
                "",
                "---- run_ks_test::foo/fail.ks stdout ----",
                "assertion failed: values differ",
                "thread 'main' panicked at ...",
                "",
                "",
                "failures:",
                "    run_ks_test::foo/fail.ks",
                "",
                "test result: FAILED.",
            ]
            .join("\n"),
        );

        assert_eq!(parser.status_of(0), Some(BatchTestStatus::Passed));
        assert_eq!(parser.status_of(1), Some(BatchTestStatus::Failed));
        assert_eq!(parser.status_of(2), Some(BatchTestStatus::Ignored));
        let msg = parser.failure_message("run_ks_test::foo/fail.ks");
        assert!(
            msg.contains("assertion failed: values differ"),
            "failure block missing content: {msg:?}"
        );
        assert!(
            msg.contains("panicked"),
            "failure block missing panic line: {msg:?}"
        );
    }

    #[test]
    fn batch_parser_attributes_json_output() {
        let names = vec![
            "run_ks_test::foo/ok.ks".to_string(),
            "run_ks_test::foo/fail.ks".to_string(),
        ];
        let mut parser = BatchParser::new(&names);

        for line in [
            r#"{ "type": "suite", "event": "started", "test_count": 2 }"#,
            r#"{ "type": "test", "event": "started", "name": "run_ks_test::foo/ok.ks" }"#,
            r#"{ "type": "test", "name": "run_ks_test::foo/ok.ks", "event": "ok" }"#,
            r#"{ "type": "test", "event": "started", "name": "run_ks_test::foo/fail.ks" }"#,
            r#"{ "type": "test", "name": "run_ks_test::foo/fail.ks", "event": "failed", "stdout": "Error: Diagnostic matching failed\n" }"#,
            r#"{ "type": "suite", "event": "failed", "passed": 1, "failed": 1 }"#,
        ] {
            parser.observe(line);
        }

        assert_eq!(parser.status_of(0), Some(BatchTestStatus::Passed));
        assert_eq!(parser.status_of(1), Some(BatchTestStatus::Failed));
        assert_eq!(parser.active_index(), None);
        assert_eq!(
            parser.failure_message("run_ks_test::foo/fail.ks"),
            "Error: Diagnostic matching failed"
        );
    }

    #[test]
    fn batch_parser_tracks_active_when_output_is_mid_test() {
        // Harness printed the line announcing the next test but hasn't yet
        // printed its result (happens when the test crashes the subprocess).
        let names = vec![
            "run_ks_test::a.ks".to_string(),
            "run_ks_test::b.ks".to_string(),
        ];
        let mut parser = BatchParser::new(&names);
        parser.observe("test run_ks_test::a.ks ... ok");
        parser.observe("test run_ks_test::b.ks ... ");
        assert_eq!(parser.status_of(0), Some(BatchTestStatus::Passed));
        assert_eq!(parser.status_of(1), None);
        assert_eq!(parser.active_index(), Some(1));
    }
}

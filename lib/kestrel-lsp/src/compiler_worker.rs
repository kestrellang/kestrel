//! Persistent `Compiler` owned by a dedicated thread.
//!
//! Why a thread (and not `Arc<Mutex<Compiler>>`): `kestrel_hecs::World`
//! holds `RefCell`s for query and accumulator storage, so it is `!Send`.
//! It cannot live behind `tokio::sync::Mutex` (which requires `Send`)
//! and cannot cross `spawn_blocking` boundaries. A single dedicated
//! thread that owns the `Compiler` end-to-end is the simplest fix and
//! also serializes access — which we'd need anyway since hECS is
//! single-threaded internally.
//!
//! Invalidation policy: stdlib loads once and is never touched again.
//! On user-side changes, only files whose content actually changed (or
//! were added/removed) are despawned and rebuilt. Unchanged files keep
//! their entities and cached query results — downstream queries (infer,
//! analyze) get automatic cache hits for bodies that didn't change.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use kestrel_compiler::Compiler;
use kestrel_hecs::Entity;
use tokio::sync::{mpsc, oneshot};

/// A unit of work to run against the persistent `Compiler`. Runs on the
/// worker thread; closes over its own `oneshot::Sender` to deliver the
/// typed reply.
type Job = Box<dyn FnOnce(&Compiler, &HashMap<String, Entity>) + Send>;

struct Request {
    /// Stdlib paths + text. Used only on the first request (and on any
    /// later request that finds the stdlib has changed — see
    /// [`sync_stdlib`]).
    stdlib_sources: Arc<HashMap<String, String>>,
    /// User paths + text for this request.
    user_sources: Arc<HashMap<String, String>>,
    job: Job,
}

/// A clonable, `Send + Sync` handle to the worker thread. Hold one per
/// `Backend` and clone it into handler tasks.
#[derive(Clone)]
pub struct CompilerHandle {
    tx: mpsc::UnboundedSender<Request>,
}

impl CompilerHandle {
    /// Spawn the worker thread and return its handle.
    ///
    /// The worker survives until the last `CompilerHandle` is dropped
    /// (the channel closes and the worker loop exits).
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        std::thread::Builder::new()
            .name("kestrel-compiler-worker".into())
            .spawn(move || run_worker(rx))
            .expect("failed to spawn compiler worker");
        Self { tx }
    }

    /// Run `f` against the persistent `Compiler`, syncing it to the
    /// given source state first. Returns `None` if the worker has
    /// shut down or the reply channel was dropped before the closure
    /// finished.
    ///
    /// The closure runs on the worker thread, so it does not need to
    /// be `Sync`. Captures and the return value must be `Send` because
    /// they cross thread boundaries via the channel.
    pub async fn with_compiler<R, F>(
        &self,
        stdlib_sources: Arc<HashMap<String, String>>,
        user_sources: Arc<HashMap<String, String>>,
        f: F,
    ) -> Option<R>
    where
        R: Send + 'static,
        F: FnOnce(&Compiler, &HashMap<String, Entity>) -> R + Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel::<R>();
        let job: Job = Box::new(move |c, by_path| {
            let result = f(c, by_path);
            let _ = reply_tx.send(result);
        });
        if self
            .tx
            .send(Request {
                stdlib_sources,
                user_sources,
                job,
            })
            .is_err()
        {
            return None;
        }
        reply_rx.await.ok()
    }
}

/// Worker-thread state. Lives for the lifetime of the worker.
struct WorkerState {
    compiler: Compiler,
    by_path: HashMap<String, Entity>,
    /// Last stdlib snapshot we built. Used to detect stdlib changes
    /// (rare — only on `kestrel.stdlibPath` config edits).
    stdlib_text: HashMap<String, String>,
    /// Last user snapshot we built. Used to detect any user-side
    /// change (path set or content).
    user_text: HashMap<String, String>,
}

impl WorkerState {
    fn fresh() -> Self {
        Self {
            compiler: Compiler::new(),
            by_path: HashMap::new(),
            stdlib_text: HashMap::new(),
            user_text: HashMap::new(),
        }
    }
}

fn run_worker(mut rx: mpsc::UnboundedReceiver<Request>) {
    let mut state = WorkerState::fresh();
    let verify = std::env::var("KESTREL_LSP_VERIFY_PERSISTENT")
        .map(|v| v == "1")
        .unwrap_or(false);
    while let Some(req) = rx.blocking_recv() {
        // Stdlib drift is the rare case (config edit). When it
        // happens, blow up the world entirely — there's no safe way
        // to swap stdlib entities out from under cached queries.
        let stdlib_reset = sync_stdlib(&mut state, &req.stdlib_sources);
        sync_user(&mut state, &req.user_sources, stdlib_reset);
        // Handler closures can panic (e.g. on unexpected compiler
        // state). Catch and log rather than killing the worker — the
        // oneshot reply_tx will be dropped, returning None to the
        // caller, which every handler already handles gracefully.
        if let Err(payload) = catch_unwind(AssertUnwindSafe(|| {
            (req.job)(&state.compiler, &state.by_path);
        })) {
            let msg = match payload.downcast_ref::<&str>() {
                Some(s) => (*s).to_string(),
                None => match payload.downcast_ref::<String>() {
                    Some(s) => s.clone(),
                    None => "unknown panic".to_string(),
                },
            };
            eprintln!("[kestrel-lsp] worker caught panic: {msg}");
        }
        if verify {
            verify_against_fresh(&state, &req.stdlib_sources, &req.user_sources);
        }
    }
}

/// Debug-only sanity check: build a fresh `Compiler` from the same
/// source set, run the standard analysis pipeline on both, and log to
/// stderr when their diagnostic outputs diverge. Off by default; gated
/// by `KESTREL_LSP_VERIFY_PERSISTENT=1`.
///
/// Catches gross drift (entities not cleaned up, accumulated state
/// stuck across rebuilds) without needing per-call closure replay. If
/// the diagnostic sets differ, the persistent path almost certainly
/// has a bug.
fn verify_against_fresh(
    state: &WorkerState,
    stdlib_sources: &HashMap<String, String>,
    user_sources: &HashMap<String, String>,
) {
    use kestrel_compiler_driver::CompilerDriver;

    let mut fresh = Compiler::new();
    let mut all_paths: Vec<&String> = stdlib_sources.keys().chain(user_sources.keys()).collect();
    all_paths.sort();
    for path in all_paths {
        let text = stdlib_sources
            .get(path)
            .or_else(|| user_sources.get(path))
            .expect("path in one of the source maps");
        let entity = fresh.set_source(path, text.clone());
        fresh.build(entity);
    }
    let driver = CompilerDriver::new(&fresh);
    let _ = driver.infer_all();
    let _ = driver.analyze_all(false);
    let fresh_diags = fresh.diagnostics();

    let driver = CompilerDriver::new(&state.compiler);
    let _ = driver.infer_all();
    let _ = driver.analyze_all(false);
    let live_diags = state.compiler.diagnostics();

    if live_diags.len() != fresh_diags.len() {
        eprintln!(
            "[KESTREL_LSP_VERIFY] diagnostic count mismatch: persistent={}, fresh={}",
            live_diags.len(),
            fresh_diags.len()
        );
        let mut live_msgs: Vec<String> = live_diags.iter().map(|d| d.message.clone()).collect();
        let mut fresh_msgs: Vec<String> = fresh_diags.iter().map(|d| d.message.clone()).collect();
        live_msgs.sort();
        fresh_msgs.sort();
        let only_live: Vec<&String> = live_msgs
            .iter()
            .filter(|m| !fresh_msgs.contains(m))
            .take(5)
            .collect();
        let only_fresh: Vec<&String> = fresh_msgs
            .iter()
            .filter(|m| !live_msgs.contains(m))
            .take(5)
            .collect();
        eprintln!("[KESTREL_LSP_VERIFY]   only-persistent (≤5): {only_live:?}");
        eprintln!("[KESTREL_LSP_VERIFY]   only-fresh      (≤5): {only_fresh:?}");
    }
}

/// Ensure the persistent `Compiler` reflects `stdlib_sources`. Returns
/// `true` if a full reset happened (caller must treat user code as
/// gone too).
///
/// Stdlib is normally loaded once on the first request. If the
/// configured stdlib changes mid-session (rare), we drop the entire
/// world and start over rather than try to surgically swap entities.
fn sync_stdlib(state: &mut WorkerState, stdlib_sources: &HashMap<String, String>) -> bool {
    if state.stdlib_text == *stdlib_sources && !state.stdlib_text.is_empty() {
        return false;
    }
    if state.stdlib_text.is_empty() && stdlib_sources.is_empty() {
        // No stdlib configured — fine, just skip.
        return false;
    }

    // Either first load or stdlib actually changed: rebuild from a
    // fresh `Compiler`. Cheaper to rebuild from scratch than to try to
    // surgically swap stdlib entities — and stdlib edits are vanishingly
    // rare.
    let was_initialized = !state.stdlib_text.is_empty();
    *state = WorkerState::fresh();

    let mut paths: Vec<&String> = stdlib_sources.keys().collect();
    paths.sort();
    for path in paths {
        let text = &stdlib_sources[path];
        let entity = state.compiler.set_source(path, text.clone());
        state.compiler.build(entity);
        state.by_path.insert(path.clone(), entity);
        state.stdlib_text.insert(path.clone(), text.clone());
    }
    was_initialized
}

/// Ensure the persistent `Compiler` reflects `user_sources`.
///
/// Incremental policy: only unbuild+rebuild files whose content
/// actually changed, plus any files that were added or removed.
/// Unchanged files keep their entities and cached query results.
///
/// `force_full_rebuild` is set by the caller when stdlib reset wiped
/// everything — in that case we have no current state to compare, so
/// we always rebuild.
fn sync_user(
    state: &mut WorkerState,
    user_sources: &HashMap<String, String>,
    force_full_rebuild: bool,
) {
    if !force_full_rebuild && state.user_text == *user_sources {
        return;
    }

    if force_full_rebuild {
        // Stdlib was reset — no surviving entities, rebuild everything.
        let current_paths: Vec<String> = state.user_text.keys().cloned().collect();
        rebuild_files(state, &current_paths, user_sources);
        return;
    }

    // Compute the minimal set of paths that need unbuild+rebuild:
    // 1. Removed: in current state but not in desired sources
    // 2. Changed/added: content differs or path is new
    let mut paths_to_unbuild: Vec<String> = Vec::new();
    let mut paths_to_build: Vec<String> = Vec::new();

    // Removed files
    for path in state.user_text.keys() {
        if !user_sources.contains_key(path) {
            paths_to_unbuild.push(path.clone());
        }
    }

    // Changed or added files
    for (path, new_text) in user_sources {
        match state.user_text.get(path) {
            Some(old_text) if old_text == new_text => {
                // Unchanged — skip, keep existing entity + cache
            }
            _ => {
                // Changed content or new file
                if state.user_text.contains_key(path) {
                    paths_to_unbuild.push(path.clone());
                }
                paths_to_build.push(path.clone());
            }
        }
    }

    if paths_to_unbuild.is_empty() && paths_to_build.is_empty() {
        return;
    }

    // Despawn invalidated file entities.
    for path in &paths_to_unbuild {
        if let Some(entity) = state.by_path.remove(path) {
            state.compiler.unbuild_file(entity);
        }
        state.user_text.remove(path);
    }

    // Bump revision so change tracking sees this batch as a unit.
    state.compiler.begin_revision();

    // Build only the new/changed files in deterministic order.
    paths_to_build.sort();
    for path in &paths_to_build {
        let text = &user_sources[path];
        let entity = state.compiler.set_source(path, text.clone());
        state.compiler.build(entity);
        state.by_path.insert(path.clone(), entity);
        state.user_text.insert(path.clone(), text.clone());
    }
}

/// Full rebuild: despawn all `paths_to_unbuild`, then rebuild every
/// file in `desired_user_sources`. Used only on forced full rebuilds
/// (stdlib reset). The incremental path in [`sync_user`] handles
/// partial rebuilds inline.
fn rebuild_files(
    state: &mut WorkerState,
    paths_to_unbuild: &[String],
    desired_user_sources: &HashMap<String, String>,
) {
    // Despawn old user file entities.
    for path in paths_to_unbuild {
        if let Some(entity) = state.by_path.remove(path) {
            state.compiler.unbuild_file(entity);
        }
        state.user_text.remove(path);
    }

    // Bump revision so change tracking sees this batch as a unit.
    state.compiler.begin_revision();

    // Build user files in deterministic order.
    let mut paths: Vec<&String> = desired_user_sources.keys().collect();
    paths.sort();
    for path in paths {
        let text = &desired_user_sources[path];
        let entity = state.compiler.set_source(path, text.clone());
        state.compiler.build(entity);
        state.by_path.insert(path.clone(), entity);
        state.user_text.insert(path.clone(), text.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::FileId;

    fn arc_map(pairs: &[(&str, &str)]) -> Arc<HashMap<String, String>> {
        Arc::new(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        )
    }

    #[tokio::test(flavor = "current_thread")]
    async fn first_request_loads_stdlib_and_user() {
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[]);
        let user = arc_map(&[("/u/main.ks", "module Main\nstruct A {}")]);

        let count = handle
            .with_compiler(stdlib, user, |compiler, by_path| {
                assert!(by_path.contains_key("/u/main.ks"));
                compiler.world().iter_component::<FileId>().count()
            })
            .await
            .unwrap();
        assert!(count > 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn second_request_unchanged_user_skips_rebuild() {
        // If user_sources is byte-equal to last call, the worker must
        // not despawn/rebuild — verify the file entity is the same.
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[]);
        let user = arc_map(&[("/u/main.ks", "module Main")]);

        let e1: Entity = handle
            .with_compiler(
                stdlib.clone(),
                user.clone(),
                |_, by_path| by_path["/u/main.ks"],
            )
            .await
            .unwrap();
        let e2: Entity = handle
            .with_compiler(stdlib, user, |_, by_path| by_path["/u/main.ks"])
            .await
            .unwrap();
        assert_eq!(e1, e2, "unchanged user must not rebuild file entity");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn changing_user_text_rebuilds_user_files() {
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[]);
        let user_v1 = arc_map(&[("/u/m.ks", "module M")]);
        let user_v2 = arc_map(&[("/u/m.ks", "module M\nstruct B {}")]);

        let e1: Entity = handle
            .with_compiler(stdlib.clone(), user_v1, |_, b| b["/u/m.ks"])
            .await
            .unwrap();
        let e2: Entity = handle
            .with_compiler(stdlib, user_v2, |_, b| b["/u/m.ks"])
            .await
            .unwrap();
        assert_ne!(e1, e2, "changed user source must allocate a new entity");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn incremental_preserves_unchanged_file_entities() {
        // When one file changes, other files' entities must survive.
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[]);
        let user_v1 = arc_map(&[
            ("/u/a.ks", "module A"),
            ("/u/b.ks", "module B"),
        ]);
        let user_v2 = arc_map(&[
            ("/u/a.ks", "module A"),
            ("/u/b.ks", "module B\nstruct X {}"),
        ]);

        let (ea1, eb1): (Entity, Entity) = handle
            .with_compiler(stdlib.clone(), user_v1, |_, b| {
                (b["/u/a.ks"], b["/u/b.ks"])
            })
            .await
            .unwrap();
        let (ea2, eb2): (Entity, Entity) = handle
            .with_compiler(stdlib, user_v2, |_, b| {
                (b["/u/a.ks"], b["/u/b.ks"])
            })
            .await
            .unwrap();
        assert_eq!(ea1, ea2, "unchanged file A must keep its entity");
        assert_ne!(eb1, eb2, "changed file B must get a new entity");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn incremental_handles_added_and_removed_files() {
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[]);
        let user_v1 = arc_map(&[
            ("/u/a.ks", "module A"),
            ("/u/b.ks", "module B"),
        ]);
        // Remove b.ks, add c.ks
        let user_v2 = arc_map(&[
            ("/u/a.ks", "module A"),
            ("/u/c.ks", "module C"),
        ]);

        let ea1: Entity = handle
            .with_compiler(stdlib.clone(), user_v1, |_, b| b["/u/a.ks"])
            .await
            .unwrap();
        let result: (Entity, bool) = handle
            .with_compiler(stdlib, user_v2, |_, b| {
                (b["/u/a.ks"], b.contains_key("/u/b.ks"))
            })
            .await
            .unwrap();
        assert_eq!(ea1, result.0, "unchanged file A must keep its entity");
        assert!(!result.1, "removed file B must be gone from by_path");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn unchanged_user_keeps_query_cache_warm() {
        // Two consecutive identical-input requests should run the parse
        // query once, not twice. Uses Compiler::query_exec_count to
        // distinguish a cache hit from a re-execution.
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[]);
        let user = arc_map(&[("/u/m.ks", "module M\nstruct A {}\nstruct B {}")]);

        let after1: u64 = handle
            .with_compiler(stdlib.clone(), user.clone(), |c, b| {
                let _ = c.parse(b["/u/m.ks"]);
                c.query_exec_count()
            })
            .await
            .unwrap();
        let after2: u64 = handle
            .with_compiler(stdlib, user, |c, b| {
                let _ = c.parse(b["/u/m.ks"]);
                c.query_exec_count()
            })
            .await
            .unwrap();
        assert_eq!(
            after1, after2,
            "second identical call must hit the parse cache"
        );
    }

    /// Mirrors the LSP first-build sequence: load stdlib, then call
    /// `begin_revision()` (the boundary we add in `rebuild_files`),
    /// then build a user file. Compare diagnostics against a "no
    /// boundary" build (CLI-style). Any divergence proves the
    /// revision boundary itself is the culprit.
    #[test]
    fn revision_boundary_does_not_change_diagnostics() {
        use kestrel_compiler::Compiler;
        use kestrel_compiler_driver::CompilerDriver;
        use std::path::Path;

        let std_dir = Path::new("/Users/dino/Documents/Projects/kestrel/lang/std");
        if !std_dir.exists() {
            return; // skip when run outside the dev tree
        }

        let user_src = "module hello\n\
                        public func add(x: Int64, y: Int64) -> Int64 { x + y }\n\
                        public struct Point { public var x: Int64; public var y: Int64 }\n\
                        public func origin() -> Point { Point(x: 0, y: 0) }\n";
        let user_path = "/tmp/repro_main.ks";

        // CLI-style: single revision.
        let cli_diags = {
            let mut c = Compiler::new();
            c.load_dir(std_dir);
            let e = c.set_source(user_path, user_src.into());
            c.build(e);
            let d = CompilerDriver::new(&c);
            let _ = d.infer_all();
            let _ = d.analyze_all(false);
            c.diagnostics()
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        };

        // LSP-style: extra begin_revision() between stdlib and user.
        let lsp_diags = {
            let mut c = Compiler::new();
            c.load_dir(std_dir);
            c.begin_revision();
            let e = c.set_source(user_path, user_src.into());
            c.build(e);
            let d = CompilerDriver::new(&c);
            let _ = d.infer_all();
            let _ = d.analyze_all(false);
            c.diagnostics()
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            cli_diags, lsp_diags,
            "extra begin_revision() between stdlib and user code changed diagnostics; cli={cli_diags:?} lsp={lsp_diags:?}"
        );
    }

    /// Reproduce the actual LSP first-build flow against the same files
    /// VS Code is opening: stdlib loaded via the worker's sync_stdlib,
    /// then user file (lang/hello/main.ks) loaded via sync_user.
    /// Compare diagnostics with a CLI-style single-Compiler build.
    #[test]
    fn worker_first_build_matches_cli_for_hello() {
        use kestrel_compiler::Compiler;
        use kestrel_compiler_driver::CompilerDriver;
        use std::path::Path;

        let std_dir = Path::new("/Users/dino/Documents/Projects/kestrel/lang/std");
        let user_path = "/Users/dino/Documents/Projects/kestrel/lang/hello/main.ks";
        if !std_dir.exists() || !Path::new(user_path).exists() {
            return; // skip when run outside the dev tree
        }

        // Walk the stdlib dir to mirror what the LSP does.
        let mut stdlib_paths: Vec<std::path::PathBuf> = Vec::new();
        crate::project::walk_kestrel_sources(std_dir, &mut stdlib_paths);
        let mut stdlib_map: HashMap<String, String> = HashMap::new();
        for p in &stdlib_paths {
            let canon = p.canonicalize().unwrap();
            stdlib_map.insert(
                canon.to_string_lossy().into_owned(),
                std::fs::read_to_string(&canon).unwrap(),
            );
        }

        let user_text = std::fs::read_to_string(user_path).unwrap();
        let mut user_map: HashMap<String, String> = HashMap::new();
        user_map.insert(user_path.to_string(), user_text.clone());

        // CLI-style baseline.
        let cli_diags = {
            let mut c = Compiler::new();
            c.load_dir(std_dir);
            let e = c.set_source(user_path, user_text.clone());
            c.build(e);
            let d = CompilerDriver::new(&c);
            let _ = d.infer_all();
            let _ = d.analyze_all(false);
            let mut msgs: Vec<String> = c.diagnostics().iter().map(|d| d.message.clone()).collect();
            msgs.sort();
            msgs
        };

        // Worker-style: route through sync_stdlib + sync_user.
        let worker_diags = {
            let mut state = WorkerState::fresh();
            sync_stdlib(&mut state, &stdlib_map);
            sync_user(&mut state, &user_map, false);
            let d = CompilerDriver::new(&state.compiler);
            let _ = d.infer_all();
            let _ = d.analyze_all(false);
            let mut msgs: Vec<String> = state
                .compiler
                .diagnostics()
                .iter()
                .map(|d| d.message.clone())
                .collect();
            msgs.sort();
            msgs
        };

        assert_eq!(
            cli_diags, worker_diags,
            "worker first-build diagnostics differ from CLI;\n  cli={cli_diags:?}\n  worker={worker_diags:?}"
        );
    }

    /// End-to-end regression: rebuild user code in the persistent
    /// compiler and confirm diagnostics still match a CLI-style fresh
    /// build. This is the path the live LSP exercises on every edit
    /// — without the hierarchy-dep fix in hECS, queries that walked
    /// the module's children list returned stale dead-entity IDs from
    /// cache, producing bogus "X is not a type" / "no member" errors
    /// referencing entities with no `Name` component.
    #[test]
    fn rebuild_user_code_matches_fresh_diagnostics() {
        use kestrel_compiler::Compiler;
        use kestrel_compiler_driver::CompilerDriver;
        use std::path::Path;

        let std_dir = Path::new("/Users/dino/Documents/Projects/kestrel/lang/std");
        if !std_dir.exists() {
            return;
        }

        let user_path = "/tmp/repro_rebuild.ks".to_string();
        let v1 = "module hello\n\
                  public func add(x: Int64, y: Int64) -> Int64 { x + y }\n\
                  public struct Point { public var x: Int64; public var y: Int64 }\n\
                  public func origin() -> Point { Point(x: 0, y: 0) }\n";
        let v2 = "module hello\n\
                  public func add(x: Int64, y: Int64) -> Int64 { x + y }\n\
                  public struct Point { public var x: Int64; public var y: Int64 }\n\
                  public func origin() -> Point { Point(x: 0, y: 0) }\n\
                  public func sum_origin() -> Int64 { let p = origin(); add(p.x, p.y) }\n";

        let mut stdlib_paths: Vec<std::path::PathBuf> = Vec::new();
        crate::project::walk_kestrel_sources(std_dir, &mut stdlib_paths);
        let mut stdlib_map: HashMap<String, String> = HashMap::new();
        for p in &stdlib_paths {
            let canon = p.canonicalize().unwrap();
            stdlib_map.insert(
                canon.to_string_lossy().into_owned(),
                std::fs::read_to_string(&canon).unwrap(),
            );
        }

        // Worker first build (v1) → edit to v2 → second sync_user.
        let mut state = WorkerState::fresh();
        sync_stdlib(&mut state, &stdlib_map);
        let mut user_v1: HashMap<String, String> = HashMap::new();
        user_v1.insert(user_path.clone(), v1.into());
        sync_user(&mut state, &user_v1, false);
        let mut user_v2: HashMap<String, String> = HashMap::new();
        user_v2.insert(user_path.clone(), v2.into());
        sync_user(&mut state, &user_v2, false);
        let driver = CompilerDriver::new(&state.compiler);
        let _ = driver.infer_all();
        let _ = driver.analyze_all(false);
        let mut worker_diags: Vec<String> = state
            .compiler
            .diagnostics()
            .iter()
            .map(|d| d.message.clone())
            .collect();
        worker_diags.sort();

        // Fresh CLI build of v2.
        let mut fresh = Compiler::new();
        fresh.load_dir(std_dir);
        let e = fresh.set_source(&user_path, v2.into());
        fresh.build(e);
        let driver = CompilerDriver::new(&fresh);
        let _ = driver.infer_all();
        let _ = driver.analyze_all(false);
        let mut fresh_diags: Vec<String> = fresh
            .diagnostics()
            .iter()
            .map(|d| d.message.clone())
            .collect();
        fresh_diags.sort();

        assert_eq!(
            worker_diags, fresh_diags,
            "rebuild after edit produced different diagnostics from fresh build;\n  worker={worker_diags:?}\n  fresh={fresh_diags:?}"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verify_against_fresh_smoke() {
        // Smoke-test the validation path: with a small program that has
        // no diagnostics, persistent and fresh must produce the same
        // (empty) diagnostic set after several edit cycles. Doesn't
        // assert anything about the verify hook output (it logs to
        // stderr); it just confirms the comparison runs without panic.
        // Exercise the underlying function directly so the env var
        // doesn't need to be set in tests.
        let mut state = WorkerState::fresh();
        let stdlib: HashMap<String, String> = HashMap::new();
        let mut user_v1 = HashMap::new();
        user_v1.insert("/u/m.ks".to_string(), "module M".to_string());
        sync_stdlib(&mut state, &stdlib);
        sync_user(&mut state, &user_v1, false);
        verify_against_fresh(&state, &stdlib, &user_v1);

        let mut user_v2 = HashMap::new();
        user_v2.insert("/u/m.ks".to_string(), "module M\nstruct A {}".to_string());
        sync_user(&mut state, &user_v2, false);
        verify_against_fresh(&state, &stdlib, &user_v2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stdlib_persists_across_user_changes() {
        // Stdlib entities must remain alive (and their query caches
        // warm) when only user code changes.
        let handle = CompilerHandle::spawn();
        let stdlib = arc_map(&[("/std/core.ks", "module Std\nstruct Std1 {}")]);
        let user_v1 = arc_map(&[("/u/m.ks", "module M")]);
        let user_v2 = arc_map(&[("/u/m.ks", "module M\nstruct U {}")]);

        let std_entity_v1: Entity = handle
            .with_compiler(stdlib.clone(), user_v1, |_, b| b["/std/core.ks"])
            .await
            .unwrap();
        let std_entity_v2: Entity = handle
            .with_compiler(stdlib, user_v2, |_, b| b["/std/core.ks"])
            .await
            .unwrap();
        assert_eq!(
            std_entity_v1, std_entity_v2,
            "stdlib must not be touched when user code changes"
        );
    }
}

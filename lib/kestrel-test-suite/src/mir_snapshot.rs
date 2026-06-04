//! MIR golden-file snapshot testing.
//!
//! Compares the MIR text output of a compiled module against a stored
//! `.mir` golden file. Set `KESTREL_UPDATE_SNAPSHOTS=1` to write new
//! golden files instead of asserting equality.

use std::path::Path;

use kestrel_mir::MirModule;
use kestrel_mir::display::{display_module, display_module_filtered};

/// Check the MIR output against a golden snapshot file.
///
/// - `test_path`: path to the `.ks` test file (snapshot stored alongside)
/// - `mir`: the lowered MIR (OSSA) module
/// - `filter`: optional function-name substring to restrict output to
/// - `snapshot_name`: optional override for the snapshot filename
pub fn check_mir_snapshot(
    test_path: &Path,
    mir: &MirModule,
    filter: Option<&str>,
    snapshot_name: Option<&str>,
) -> Result<(), String> {
    let actual = match filter {
        Some(func_name) => {
            let rendered = display_module_filtered(mir, func_name);
            if rendered.trim().is_empty() {
                let available: Vec<&str> =
                    mir.functions.values().map(|f| f.name.as_str()).collect();
                return Err(format!(
                    "Function matching '{}' not found in MIR. Available: {:?}",
                    func_name, available
                ));
            }
            rendered
        },
        None => display_module(mir),
    };

    let snapshot_dir = test_path.parent().unwrap().join("snapshots");
    let name = snapshot_name.unwrap_or_else(|| test_path.file_stem().unwrap().to_str().unwrap());
    let snapshot_path = snapshot_dir.join(format!("{}.mir", name));

    // Update mode: write actual output as new golden file
    if std::env::var("KESTREL_UPDATE_SNAPSHOTS").is_ok() {
        std::fs::create_dir_all(&snapshot_dir)
            .map_err(|e| format!("failed to create snapshot dir: {e}"))?;
        std::fs::write(&snapshot_path, &actual)
            .map_err(|e| format!("failed to write snapshot: {e}"))?;
        return Ok(());
    }

    // Check mode: compare against existing golden file
    if !snapshot_path.exists() {
        return Err(format!(
            "No snapshot file at {}.\n\
             Run with KESTREL_UPDATE_SNAPSHOTS=1 to create.\n\n\
             Actual MIR:\n{}",
            snapshot_path.display(),
            actual
        ));
    }

    let expected = std::fs::read_to_string(&snapshot_path)
        .map_err(|e| format!("failed to read snapshot: {e}"))?;

    if actual.trim() == expected.trim() {
        Ok(())
    } else {
        Err(format!(
            "MIR snapshot mismatch for {}\n\n\
             --- expected ---\n{}\n\n\
             --- actual ---\n{}\n\n\
             Run with KESTREL_UPDATE_SNAPSHOTS=1 to update.",
            snapshot_path.display(),
            expected.trim(),
            actual.trim()
        ))
    }
}

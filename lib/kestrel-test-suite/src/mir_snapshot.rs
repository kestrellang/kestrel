//! MIR golden-file snapshot testing.
//!
//! Compares the MIR text output of a compiled module against a stored
//! `.mir` golden file. Set `KESTREL_UPDATE_SNAPSHOTS=1` to write new
//! golden files instead of asserting equality.

use std::path::Path;

use kestrel_mir_2::MirModule;

/// Check the MIR output against a golden snapshot file.
///
/// - `test_path`: path to the `.ks` test file (snapshot stored alongside)
/// - `mir`: the lowered MIR module
/// - `filter`: optional function name to extract (by `FunctionDef.name`)
/// - `snapshot_name`: optional override for the snapshot filename
pub fn check_mir_snapshot(
    test_path: &Path,
    mir: &MirModule,
    filter: Option<&str>,
    snapshot_name: Option<&str>,
) -> Result<(), String> {
    let actual = if let Some(func_name) = filter {
        extract_function_mir(mir, func_name)?
    } else {
        format!("{}", mir.display())
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

/// Extract the MIR display text for a single function by name.
///
/// Renders the full module, then finds the `func @<name>` block and
/// returns everything from that line through its closing `}`.
fn extract_function_mir(mir: &MirModule, func_name: &str) -> Result<String, String> {
    let full = format!("{}", mir.display());

    // Find the function by matching `func @<name>` or `func @....<name>`
    let mut result_lines: Vec<&str> = Vec::new();
    let mut inside = false;
    let mut brace_depth: i32 = 0;

    for line in full.lines() {
        if !inside {
            let trimmed = line.trim();
            if trimmed.starts_with("func @") {
                // Extract the function name from `func @<name>(...`
                let after_at = &trimmed[6..];
                let name_end = after_at
                    .find(|c: char| c == '(' || c == '[' || c == ' ')
                    .unwrap_or(after_at.len());
                let name = &after_at[..name_end];
                if name == func_name || name.ends_with(&format!(".{}", func_name)) {
                    inside = true;
                    brace_depth = line.chars().filter(|&c| c == '{').count() as i32
                        - line.chars().filter(|&c| c == '}').count() as i32;
                    result_lines.push(line);
                    if brace_depth == 0 {
                        // Single-line (extern or bodyless) — done
                        break;
                    }
                    continue;
                }
            }
        } else {
            brace_depth += line.chars().filter(|&c| c == '{').count() as i32;
            brace_depth -= line.chars().filter(|&c| c == '}').count() as i32;
            result_lines.push(line);
            if brace_depth <= 0 {
                break;
            }
        }
    }

    if result_lines.is_empty() {
        let available: Vec<&str> = mir.functions.iter().map(|f| f.name.as_str()).collect();
        Err(format!(
            "Function '{}' not found in MIR. Available: {:?}",
            func_name, available
        ))
    } else {
        Ok(result_lines.join("\n"))
    }
}

//! Statement lowering — stub for Phase 4.

use kestrel_hir::body::HirStmtId;

use super::BodyCtx;

impl BodyCtx<'_, '_> {
    /// Lower an HIR statement. Stub — no-op for now.
    pub fn lower_stmt(&mut self, _stmt_id: HirStmtId) {
        // Will be implemented in Phase 4
    }
}

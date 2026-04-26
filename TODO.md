# Kestrel TODO — Phase 15 Completion

## Remaining Phase 15 Items

### Parser Refactor
- [ ] Integrate parser-2 into compilation pipeline (exists in worktree, not yet wired up)
- [ ] Migrate all tests to parser-2
- [ ] Remove old parser

### Flock Package Registry
- [x] Implement `RegistrySource` (conforming to `PackageSource` protocol)
- [x] Add registry URL config to flock.toml manifest
- [x] Integrate Swoop HTTP client for registry communication
- [x] Define registry API format (JSON endpoints for search, metadata, download)
- [x] Add local package cache (~/.kestrel/registry or similar)
- [x] Implement version resolution for registry deps (constraint solver for transitive deps)
- [x] Add lock file (flock.lock) for reproducible builds
- [x] Add `flock publish` command
- [x] Authentication (API tokens for publish)

### Incremental Compilation
- [ ] File-level dependency tracking
- [ ] Artifact caching between compilation runs
- [ ] Incremental semantic model reuse
> Note: Stage-based early exit already works. True incremental is a bigger effort — consider post-announcement.

### Jessup Version Manager
- [x] Design and implement
> Implemented as a Kestrel binary in `lang/jessup/`

### LSP Polish
- [ ] Surface doc comments in hover tooltips
- [ ] Any other polish items

## Announcement Checklist
- [ ] All critical Phase 15 items done
- [x] Flock registry at least MVP-functional
- [ ] LSP stable with doc comment support
- [x] README / landing page / docs site
- [ ] Example projects
- [ ] Getting started guide

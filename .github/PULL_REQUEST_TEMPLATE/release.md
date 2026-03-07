## Release v<!-- VERSION -->

<!-- Replace VERSION above with the version being released, e.g. 0.2.0 -->

## Version bump

- **Previous version:** <!-- e.g. 0.1.1 -->
- **New version:** <!-- e.g. 0.2.0 -->
- **Type:** <!-- MAJOR / MINOR / PATCH -->

## What's in this release?

<!-- Summarise what changed since the last release. You'll paste this into the GitHub Release notes after publishing. -->

### Added
-

### Fixed
-

### Changed
-

---

## Release checklist

**Version**

- [ ] `Cargo.toml` version has been bumped
- [ ] New version is strictly higher than the last published release
- [ ] New version follows semver (`MAJOR.MINOR.PATCH`)
- [ ] Branch is named `release/vX.Y.Z` matching the version

**Code**

- [ ] Compiles cleanly (`cargo build --release`)
- [ ] No debug code or `println!` statements left in
- [ ] No API keys or secrets in the diff

**Database**

- [ ] If the schema changed, all migrations are present and registered in `migrations/mod.rs`

**Testing**

- [ ] Add podcast, play episode, sync all tested manually
- [ ] OPML import and export tested
- [ ] Settings save and reload correctly
- [ ] Notes panel opens, creates, edits, and deletes correctly

**Process**

- [ ] This PR targets `main`
- [ ] Will be merged with squash and merge

---

## Post-merge steps (admin only)

After this PR is squash-merged, run:

```bash
git checkout main && git pull
git tag vX.Y.Z
git push origin vX.Y.Z
```

Then review and publish the draft release from the [Releases](../../releases) page.
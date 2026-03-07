# Releasing RCast

Only repository admins can publish releases. No release happens automatically without a deliberate human action at each
step.

---

## Overview

```
feature / fix branches
        ↓  PR → main  (CI checks compilation on code changes)

admin creates release/vX.Y.Z branch
        ↓  bumps Cargo.toml version
        ↓  PR → main  (CI validates semver + checks version is higher than last release)

PR reviewed, squash-merged into main
        ↓
admin pushes git tag vX.Y.Z on the merge commit
        ↓
GitHub Actions builds Windows + macOS binaries
        ↓
Draft release created — admin writes notes, publishes
```

---

## Versioning

RCast follows [Semantic Versioning](https://semver.org/): `MAJOR.MINOR.PATCH`

- **PATCH** — bug fixes that don't change behaviour
- **MINOR** — new features, backwards compatible
- **MAJOR** — significant product shift or breaking changes

**CI enforces automatically:**

- Version in `Cargo.toml` must be valid semver
- New version must be strictly higher than the last published release
- Git tag must exactly match `Cargo.toml` (tag `v0.2.0` requires `version = "0.2.0"`)
- Branch name must match the version (e.g. `release/v0.2.0`)

Any failure blocks the PR from merging.

---

## Step by step

### 1. Create a release branch

```bash
git checkout main && git pull
git checkout -b release/v0.2.0
```

Branch must follow the pattern `release/vX.Y.Z` exactly — the validation workflow only runs on branches matching this
pattern.

### 2. Bump the version in Cargo.toml

```toml
[package]
version = "0.2.0"
```

### 3. Open a PR into main

Title it `Release v0.2.0` and select the **Release** PR template when prompted. CI will run automatically and validate
the version. Fix any failures before requesting review.

### 4. Review and squash-merge

At least one other maintainer reviews the PR and works through the checklist. Merge using **squash and merge**.

### 5. Push the release tag

```bash
git checkout main && git pull
git tag v0.2.0
git push origin v0.2.0
```

Only admins can push `v*` tags — GitHub's tag protection rules reject this from anyone else. This tag push is what
triggers the build.

### 6. Publish the draft release

GitHub Actions builds both binaries and creates a draft release. Go to the [Releases](../../releases) page, open the
draft, write release notes, and click **Publish release**.

---

## Supported platforms

| Platform          | Binary              | Notes                            |
|-------------------|---------------------|----------------------------------|
| Windows (x86_64)  | `rcast-windows.exe` | Built on `windows-latest`        |
| macOS (Universal) | `rcast-macos`       | Intel + Apple Silicon via `lipo` |
| Linux             | —                   | Not currently distributed        |
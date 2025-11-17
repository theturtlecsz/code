# Release Process

Versioning, changelog, and publishing workflow.

---

## Versioning

**Scheme**: Semantic Versioning (SemVer)

**Format**: `MAJOR.MINOR.PATCH`

- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes

---

## Release Workflow

### 1. Prepare Release

**Update version** (`codex-cli/package.json`):
```json
{
  "version": "1.2.3"
}
```

**Update Changelog** (`CHANGELOG.md`):
```markdown
## [1.2.3] - 2025-11-17

### Added
- Dark mode support

### Fixed
- Database connection timeout

### Changed
- Improved error messages
```

---

### 2. Tag Release

```bash
git tag -a v1.2.3 -m "Release v1.2.3"
git push origin v1.2.3
```

---

### 3. GitHub Actions

**Triggers**: Push to `main` or tag push

**Jobs**:
1. Build (Linux, macOS, Windows)
2. Test (all platforms)
3. Publish to npm

**Workflow**: `.github/workflows/release.yml`

---

### 4. Verify Release

**npm**:
```bash
npm view @just-every/code version
# Should show: 1.2.3
```

**GitHub**:
- Check release notes
- Verify binaries attached

---

## Homebrew Formula

**Update formula** (`homebrew-tap/Formula/code.rb`):
```ruby
class Code < Formula
  desc "Fast local coding agent"
  homepage "https://github.com/theturtlecsz/code"
  version "1.2.3"
  # ... download URLs, SHA256
end
```

**Generate**:
```bash
bash scripts/generate-homebrew-formula.sh v1.2.3
```

---

## Changelog Generation

**Manual**:
```markdown
## [1.2.3] - 2025-11-17

### Added
- List new features

### Fixed
- List bug fixes

### Changed
- List changes
```

**Automated** (future):
```bash
# Generate from git commits
git-cliff --tag v1.2.3 > CHANGELOG.md
```

---

## Release Checklist

- [ ] Update version in package.json
- [ ] Update CHANGELOG.md
- [ ] Run full test suite (`cargo test --workspace`)
- [ ] Build release (`cargo build --release`)
- [ ] Create git tag (`git tag -a v1.2.3`)
- [ ] Push tag (`git push origin v1.2.3`)
- [ ] Verify CI passes
- [ ] Check npm publish
- [ ] Update Homebrew formula
- [ ] Create GitHub release notes

---

## Summary

**Process**:
1. Update version + changelog
2. Tag release
3. Push (CI auto-publishes)
4. Update Homebrew formula
5. Verify release

**Workflow**: `.github/workflows/release.yml`

**References**:
- SemVer: https://semver.org/
- Changelog: `CHANGELOG.md`

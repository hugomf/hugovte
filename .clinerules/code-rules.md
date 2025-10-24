# yaml-language-server: $schema=https://json.schemastore.org/clinerules.json
# .clinerules – concise mandatory rules for Cline
# Version: 2025-10-09.2  (YYYY-MM-DD.increment)
# Emergency brake: if any rule conflicts with security, common-sense, or a lead's explicit instruction, STOP and ask—rules are ordered by risk, not religion.

# 0. Session start
- Read IMPLEMENTATION_PLAN.md first. 
- Share mini-plan before any code.
- Reminders: Read .clinerules at start of each session. Follow YAGNI—code only what's needed NOW. Tests must pass before committing. NEVER push to remote—commit locally only. Stage specific files with `git add <file>`, not `git add .`. Security/crypto code needs review. Public APIs must be documented. Breaking changes require CHANGELOG updates. Memory safe code only—no unsafe blocks without review. Prefer thiserror for custom error types.

# 1. Git safety (NEVER push)
- Commit locally only.
- Conventional message: `feat|fix|refactor|docs|test|chore: desc`.
- Never run: `git push`, `git merge`, `git rebase`, `git add .`.
- Stage explicit files: `git add <file>`.

# 2. Time-box
Max time per task: 45 min. If not done, push branch, open draft PR, and hand-off to human.

# 3. Pre-commit checks (MUST pass)
pre_commit_checks:
  rust: |
    cargo check --workspace --all-targets && \
    cargo clippy --workspace --all-targets -- -D warnings && \
    cargo fmt --check && \
    cargo test --workspace
  go: |
    go vet ./... && \
    golangci-lint run && \
    test -z "$(gofmt -l .)" && \
    go test ./...
  java: "mvn spotless:apply && mvn test"
  dart: "dart format --set-exit-if-changed . && flutter analyze && flutter test"
Success: All commands exit 0; else, flag for human review.

# 4. Naming
naming_conventions:
  entities: "suffix with Entity"      # users → UserEntity
  api_models: "suffix with Api"       # UserApi
  network_models: "suffix with Network"  # NetworkUser

# 5. Code rules
- Never commit or push any code with compilation errors (run cargo check).
- Make sure the examples compile.
- Before git commit we should run the test cases (run ./test_all.sh) 
- YAGNI: code only what is needed NOW.
- One logical change per commit.
- Every code change (feature, bug fix, refactor) must be covered by tests: add new test cases or modify existing ones to verify the change.
- No `unwrap()` / `expect()` / `panic()` in production paths.
- Public APIs need rustdoc/godoc/javadoc.
- Breaking changes → update CHANGELOG.md.
- Dead code TTL: 24 h (mark `// TODO(remove-in-next-pr)`).

# 6. Warning classification
warning_classification:
  errors_blocking: true
  warnings_fatal:          # always block
    - clippy::unwrap_used
    - clippy::todo
    - rustc::unsafe_op_in_unsafe_fn
    - clippy::missing_docs_in_public_items
  warnings_allowed:        # may commit
    - clippy::style
    - clippy::pedantic

# 7. Ask human before
- Deleting any file.
- Modifying schema/migration files.
- Creating >10 files or new dependencies.
- Any unsafe/rustcrypto code.

# 8. Critical files that always need human review
critical_files:
  - IMPLEMENTATION_PLAN.md
  - TECHNICAL.md
  - README.md
  - "**/schema.sql"
  - "**/migrations/*"

# 9. Performance & security gates
perf_budget:
  instruction: "No >5 % regression in benches inside benches/"
  command: "cargo bench --bench main | tee new.txt && cargo benchcmp old.txt new.txt"  # old.txt from prior baseline run; create if missing
Success: Regression <5%; else, flag for review.

dependency_check: "cargo deny check && cargo audit --json | tee cargo-audit.json"
Success: No high/critical vulns; else, flag for human.

review_required:
  crypto: "@crypto-lead"
  db-schema: "@dba"
  ci-change: "@devops-lead"
  ">10 files": "@tech-lead"

# 10. After commit message
after_commit_message: "✅ Committed locally. Human will push when ready."

# 11. Update the Implementation Plan
- Update progress: Mark task(s) completed.
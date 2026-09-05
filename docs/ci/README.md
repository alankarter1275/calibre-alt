# Enabling GitHub Actions for Kalam

The Arena coding agent **cannot push** files under `.github/workflows/` unless the
GitHub App has the `workflows` permission. The workflow body lives here instead:

**Canonical file:** [`github-actions-ci.yml`](./github-actions-ci.yml)

## ACTION NEEDED (2026-09-04, third attempt) — rustfmt must stop pushing

Sorry, one more copy of the workflow. My previous fix was the wrong shape.

**What happened.** The rustfmt step auto-committed the formatting fix and
pushed it. That push kept being rejected, and because the step had no
`continue-on-error`, **a rejected push failed the entire build** — four runs in
a row, none of them caused by the code. Adding a rebase (attempt two) did not
help, so the collision was not the only problem, and I could not see the real
error: the sandbox cannot download Actions logs.

**The fix is to stop pushing from that step at all.** Formatting is not worth
failing a build over, and a step whose failure mode is unrelated to what it
checks sends every investigation to the wrong place. It now:

- formats, and **reports** the diff instead of committing it
- restores the tree afterwards, so clippy and the build see the real source
- never fails the run
- publishes the diff to `ci-logs/rustfmt-latest.diff` in a **separate** step
  that is `continue-on-error`, so even a failed publish cannot break anything

The agent then applies the formatting itself in the next commit, which is
honest anyway: formatting belongs in the change that caused it, not in a
drive-by commit from CI.

Copy [`github-actions-ci.yml`](./github-actions-ci.yml) over
`.github/workflows/ci.yml` and push.

---

## ACTION NEEDED (2026-09-04) — small follow-up to the rustfmt step

Low priority; nothing is broken. The report-only rustfmt step works, but on a
**clean** run it deletes `ci-logs/rustfmt-latest.diff` locally and never
commits the deletion — so the stale diff from an earlier untidy run stays in
the repository and looks like an outstanding complaint. It already fooled me
once.

Copy [`github-actions-ci.yml`](./github-actions-ci.yml) over
`.github/workflows/ci.yml` when convenient. A clean run now writes
`clean` plus the run id instead of deleting the file, so the published diff
always describes the latest run.

---

## ACTION NEEDED (2026-09-04) — publish the CI logs on success too

Low priority; nothing is broken, but the current behaviour is actively
misleading.

`ci-logs/test-latest.txt` and `ci-logs/clippy-latest.txt` are only written when
that step **fails**. So after a failure is fixed, the old failing log stays
committed and a later green run still shows red. It fooled me three separate
times — most recently reading "1 failed" from a run two hours dead while the
current run was green.

Copy [`github-actions-ci.yml`](./github-actions-ci.yml) over
`.github/workflows/ci.yml`. Both steps now run on success as well, so the
published file always describes the latest run. Same fix as the rustfmt diff.

---

## Done: windowed-grid measurement (applied 2026-09-04)

Installed and running. The `scale` job runs the 2,000-book library twice, once
with each grid, and publishes a side-by-side summary to
`ci-logs/scale-2000-comparison.txt`. Both runs happen in the same job on the
same machine on purpose: comparing against a number from a previous run would
be comparing two different rented VMs, which is what made timing-based tests
useless (see the A0 step 7 entry in the roadmap).

**No reinstall needed** even though the windowed grid later became the default.
The version installed passes `WINDOWED=1` on one run and nothing on the other;
"nothing" would now mean *windowed* as well, so both runs would measure the
same thing and report a green, meaningless comparison. Rather than ask for
another manual install, `screenshot.sh` now infers the baseline from the output
directory: a run writing to a plain `ci-shots-*` path is the old grid unless
told otherwise. An explicit `WINDOWED=` still wins, so the updated copy in this
directory works too.

---

## Done: `Cargo.lock` step (applied 2026-09-04)

The lockfile step is installed and has run — `Cargo.lock` is committed and the
dependency graph is pinned. Nothing to do here; kept as a record.

## Working agreement (manual CI handoff)

The Arena agent **cannot** create or update `.github/workflows/*` (GitHub App
has no `workflows` permission, and that cannot be toggled from your side).

So we do this forever:

| Situation | Who | What |
|-----------|-----|------|
| Need a workflow change | Agent | Writes the full file under `docs/ci/` and gives you paste/copy steps |
| Apply workflow change | **You** | Copy into `.github/workflows/ci.yml` (CLI or GitHub UI) and push |
| CI fails | Agent | Says which run/step failed |
| Share the failure | **You** | Paste the failed step log (or the “Diff in …” / rustc error block) |
| Fix code | Agent | Pushes code fixes (not workflow files) |

You already enabled CI once (Option C). Good — leave it.

> **Why the CLI path needs a special token:** GitHub refuses pushes that touch
> `.github/workflows/*` unless the Personal Access Token has the **`workflow`**
> scope (fine-grained tokens need **Workflows: Read and write**). Without it
> you get:
> `! [remote rejected] ... refusing to allow a Personal Access Token to create
> or update workflow .github/workflows/ci.yml without 'workflow' scope`.
> The commit is still created **locally**, but the push fails.
>
> Two fixes: (a) give your token the `workflow` scope (Settings → Developer
> settings → Personal access tokens → regenerate with `workflow` checked), or
> (b) use the GitHub web UI path below — no token involved.
>
> If a local commit was created but the push was rejected, and you then applied
> the same file via the web UI, the local commit is a duplicate: discard it with
> `git reset --hard origin/<branch>` (check first with
> `git log origin/<branch>..HEAD --oneline`).

### If the agent asks you to update the workflow

```bash
cd /path/to/calibre-alt
git fetch origin
git checkout arena/01a05974-calibre-alt
git pull
cp docs/ci/github-actions-ci.yml .github/workflows/ci.yml
git add .github/workflows/ci.yml
git commit -m "ci: update workflow"
git push origin arena/01a05974-calibre-alt
```

Or GitHub UI: edit `.github/workflows/ci.yml` and paste the contents of
`docs/ci/github-actions-ci.yml`.

> Note: if the branch name is not `arena/01a05974-calibre-alt`, run
> `git branch --show-current` and substitute it.

## What CI does

On every push / PR:

1. Install `libgtk-4-dev` + `libadwaita-1-dev` on `ubuntu-latest`
2. `cargo fmt --check` (auto-fix + push when it differs)
3. `cargo clippy --all-targets` (warnings do not fail; errors publish the
   diagnostics to `ci-logs/clippy-latest.txt` and fail the run)
4. `cargo test --all-targets` — compiles **and runs** the unit tests
   (in-memory SQLite, headless). Failures publish the diagnostics to
   `ci-logs/test-latest.txt` and fail the run.
5. `cargo build` and `cargo build --release`

No display / no GUI smoke tests — those stay on your Arch box at phase end.

## After it’s enabled

You don’t need to do anything else for CI. The agent watches failures, fixes
them, and only asks you to run the app when a **phase** is done.

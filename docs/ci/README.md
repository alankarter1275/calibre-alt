# Enabling GitHub Actions for Kalam

The Arena coding agent **cannot push** files under `.github/workflows/` unless the
GitHub App has the `workflows` permission. The workflow body lives here instead:

**Canonical file:** [`github-actions-ci.yml`](./github-actions-ci.yml)

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

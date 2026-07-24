# Enabling GitHub Actions for Kalam

The Arena coding agent **cannot push** files under `.github/workflows/` unless the
GitHub App has the `workflows` permission. The workflow body lives here instead:

**Canonical file:** [`github-actions-ci.yml`](./github-actions-ci.yml)

## What you need to do (once)

Pick **one** of these:

### Option A — Copy on your machine (simplest)

```bash
cd /path/to/calibre-alt
git checkout arena/019f9529-calibre-alt
git pull

mkdir -p .github/workflows
cp docs/ci/github-actions-ci.yml .github/workflows/ci.yml

git add .github/workflows/ci.yml
git commit -m "ci: enable GitHub Actions compile workflow"
git push origin arena/019f9529-calibre-alt
```

Then open the **Actions** tab on GitHub and confirm the **CI** run is green.

### Option B — Grant the Arena GitHub App `workflows` permission

In the GitHub App / installation settings used by Arena, allow **Workflows:
Read and write**. After that, ask the agent to move the file back to
`.github/workflows/ci.yml` and push — future CI edits won’t need you.

### Option C — GitHub UI

1. Repo → **Add file** → **Create new file**
2. Path: `.github/workflows/ci.yml`
3. Paste contents of `docs/ci/github-actions-ci.yml`
4. Commit to this branch

## What CI does

On every push / PR:

1. Install `libgtk-4-dev` + `libadwaita-1-dev` on `ubuntu-latest`
2. `cargo fmt --check`
3. `cargo clippy -D warnings`
4. `cargo build` and `cargo build --release`

No display / no GUI smoke tests — those stay on your Arch box at phase end.

## After it’s enabled

You don’t need to do anything else for CI. The agent watches failures, fixes
them, and only asks you to run the app when a **phase** is done.

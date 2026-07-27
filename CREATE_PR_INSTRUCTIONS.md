# How to Create a Pull Request

> A step-by-step guide for contributors opening pull requests against the Soroban Registry.

---

## Prerequisites

Before creating a PR, make sure you've completed these steps:

- [ ] Forked and cloned the repository ([see CONTRIBUTING.md](CONTRIBUTING.md#getting-started))
- [ ] Created a feature branch following the [naming conventions](CONTRIBUTING.md#feature-branch-naming)
- [ ] Made atomic, well-messaged commits ([commit format guide](CONTRIBUTING.md#commit-message-format))
- [ ] All tests pass locally (`cargo test` / `pnpm test`)
- [ ] Linting passes (`cargo clippy`, `cargo fmt --check`, `pnpm lint`)
- [ ] Your branch is rebased on the latest `main`

---

## Step 1 — Push Your Branch

```bash
# Sync with upstream first
git fetch upstream
git rebase upstream/main

# Push to your fork
git push origin feature/issue-<NUMBER>-<short-description>
```

---

## Step 2 — Open the Pull Request

### Option A: GitHub Web UI (Recommended)

1. Navigate to [the repository](https://github.com/ALIPHATICHYD/Soroban-Registry).
2. You'll see a banner: **"Compare & pull request"** — click it.
   - If the banner doesn't appear, go to **Pull requests → New pull request** and select your branch.
3. GitHub will **automatically load the PR template** with sections to fill in.
4. Complete every section of the template — the more detail you provide, the faster the review.
5. Set **Base**: `main` and **Head**: your feature branch.
6. Add labels (e.g., `feature`, `fix`, `docs`, `enhancement`).
7. Request reviewers if you know who should review.
8. Click **"Create pull request"**.

### Option B: GitHub CLI

```bash
# The --fill flag will use the PR template automatically
gh pr create \
  --base main \
  --head feature/issue-<NUMBER>-<short-description> \
  --title "feat: <Short description of your change>" \
  --fill
```

> **Tip:** If you want to edit the body interactively, omit `--fill` and the CLI will open your default editor with the template pre-loaded.

---

## Step 3 — Fill In the PR Template

When you open a PR, GitHub auto-populates the description with our [PR template](/.github/pull_request_template.md). Here's a quick overview of each section:

| Section | What to Write |
|---------|---------------|
| **Summary** | A concise explanation of *what* and *why*. |
| **Changes Made** | Bullet list of specific code, API, or schema changes. |
| **Related Issue** | Link to the issue this PR resolves (e.g., `Closes #42`). |
| **Type of Change** | Check the box that applies (feature, bug fix, docs, etc.). |
| **How Has This Been Tested?** | Describe the testing you performed (unit, integration, manual). |
| **Screenshots / Recordings** | Attach visuals for any UI changes. |
| **Checklist** | Confirm you've met all quality requirements before requesting review. |

---

## Step 4 — After Submitting

1. **CI Pipeline** — GitHub Actions will automatically run tests, linting, and builds. Fix any failures.
2. **Code Review** — A maintainer will review your PR. Respond to feedback promptly.
3. **Iterate** — Push follow-up commits to the same branch to address review comments.
4. **Merge** — Once approved and CI is green, a maintainer will merge your PR.
5. **Clean Up** — Delete your feature branch after the merge:
   ```bash
   git branch -d feature/issue-<NUMBER>-<short-description>
   git push origin --delete feature/issue-<NUMBER>-<short-description>
   ```

---

## PR Best Practices

| Practice | Details |
|----------|---------|
| **One feature per PR** | Keep PRs focused — don't bundle unrelated changes. |
| **Keep it small** | Aim for 200–500 lines when possible. Smaller PRs get reviewed faster. |
| **Link the issue** | Always include `Closes #<issue-number>` so the issue auto-closes on merge. |
| **Write clear titles** | Use the conventional format: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:` |
| **Describe your testing** | Reviewers want to know *how* you verified correctness. |
| **Update docs** | If your change adds or modifies behavior, update the relevant documentation. |
| **No merge conflicts** | Rebase on `main` before requesting review. |

---

## Troubleshooting

<details>
<summary><strong>I don't see the PR template when opening a PR</strong></summary>

The template lives at `.github/pull_request_template.md`. If GitHub doesn't auto-load it:
- Make sure the file is committed on the `main` branch.
- Try appending `?template=pull_request_template.md` to the URL.
</details>

<details>
<summary><strong>CI is failing but tests pass locally</strong></summary>

- Check you're using the correct Rust toolchain (`rust-toolchain.toml`).
- Ensure your `.env` or environment variables match CI expectations.
- Review the CI logs for the specific step that failed.
</details>

<details>
<summary><strong>Merge conflicts after rebase</strong></summary>

```bash
git fetch upstream
git rebase upstream/main
# Resolve conflicts in each file, then:
git add .
git rebase --continue
git push --force-with-lease origin feature/issue-<NUMBER>-<short-description>
```
</details>

---

## Quick Reference

```text
Fork → Clone → Branch → Code → Test → Push → PR → Review → Merge → Clean Up
```

For the full contribution workflow, see [CONTRIBUTING.md](CONTRIBUTING.md).

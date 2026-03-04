# Git & PR ワークフロー

## ブランチ

- **必ず `origin/master` から作成する**
  ```bash
  git fetch origin master
  git checkout -b {type}/{short-description} origin/master
  ```
- 既存ブランチから作った場合は push 前に `git rebase origin/master`
- 命名例: `issue-98-update-dependencies`, `feat/add-validation`

## コミット

Conventional Commits 形式:

```
{type}: {description}

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
```

type: `feat` / `fix` / `docs` / `test` / `chore` / `style` / `refactor`

- amend は避ける。修正は新コミットで対応
- 1 issue = 1 PR を原則とする

## PR 作成

```bash
git push -u origin {branch}
gh pr create --title "{type}: {description}" --body "Closes #{issue}..."
```

## PR 作成後の CI 監視

**PR 作成後は必ず CI の完了を監視する。**

```bash
gh pr checks {pr-number} --watch
```

失敗時:
```bash
gh pr checks {pr-number}             # 失敗ジョブ確認
gh run view {run-id} --log-failed    # エラーログ取得
```

修正 → 新コミット → push → 再監視のサイクルを CI が全パスするまで繰り返す。

## issue 連携

- PR 本文に `Closes #{issue}` を含める (マージ時に自動クローズ)

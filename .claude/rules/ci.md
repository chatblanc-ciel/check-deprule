# CI パイプライン

## ワークフロー一覧

| ファイル | トリガー | 内容 |
|---------|---------|------|
| `ci-check-test.yaml` | push (master), PR | fmt + clippy + build + test (3 OS) |
| `audit.yaml` | 3日ごと, Cargo.toml/lock 変更 | `cargo-audit` セキュリティ監査 |
| `publish.yaml` | タグ `*.*.*` | crates.io 公開 |
| `version-upgrade.yaml` | 手動 dispatch | バージョン更新 + PR 自動作成 |

## CI Check ジョブの内容

1. `cargo fmt --all -- --check`
2. `clippy` (reviewdog, `filter_mode: nofilter`, `fail_on_error: true`)
3. `cargo build`

**重要**: CI は `actions-rust-lang/setup-rust-toolchain@v1` で**最新 stable Rust** を使う。
ローカルの Rust バージョンが古いと CI で新しい clippy ルールに引っかかる可能性がある。
PR 前に `rustup update stable` でローカルを最新にしておくこと。

## CI Test ジョブの内容

- Linux: `cargo llvm-cov nextest` (カバレッジ付き、octocov で PR コメント)
- macOS/Windows: `cargo nextest run`

## CI 環境変数

```yaml
CARGO_TERM_COLOR: always
CARGO_INCREMENTAL: 0
CARGO_PROFILE_DEV_DEBUG: 0
RUST_BACKTRACE: short
RUSTFLAGS: -D warnings
```

`RUSTFLAGS: -D warnings` により、全 warning がコンパイルエラーになる。

## よくある CI 失敗

1. **clippy の新ルール**: CI の Rust が新しく、ローカルにないルールが適用される
   - 対処: `rustup update stable` → ローカルで `cargo clippy -- -D warnings` → 修正
2. **ブランチベース不一致**: `origin/master` と差分があるとマージ結果で CI が壊れる
   - 対処: `git rebase origin/master` → `git push --force-with-lease`

## ローカル検証コマンド (CI 相当)

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo build && cargo test
```

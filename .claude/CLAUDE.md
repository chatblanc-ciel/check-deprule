# check-deprule

Cargo workspace の依存関係制約を検証する Rust CLI ツール。
`dependency_rules.toml` に定義したルールに違反するパッケージを検出し、赤字で表示する。

## Quick Reference

- **Edition**: Rust 2024
- **License**: MIT OR Apache-2.0
- **Main branch**: `master`
- **CI**: `cargo fmt` → `clippy` (reviewdog) → `cargo build` → `cargo nextest run`
- **Publish**: crates.io (tag トリガー)

## Project Structure

```
src/
├── main.rs                          # CLI entry point
├── lib.rs                           # handler, ReturnStatus
├── metadata/mod.rs                  # cargo metadata 収集
├── dependency_rule/
│   ├── mod.rs                       # DependencyRules 構造体
│   └── rules_parser.rs             # TOML パース
└── dependency_graph/
    ├── mod.rs                       # Graph 構築
    ├── tree.rs                      # ツリー表示・違反検出
    └── formatter/
        ├── mod.rs                   # Pattern/Chunk 表示
        └── parse.rs                 # フォーマット文字列パーサ
```

## Commands

```bash
# ビルド & チェック
cargo fmt --check
cargo clippy -- -D warnings
cargo build
cargo test

# CI と同等の検証 (PR 前に必ず実行)
cargo fmt --check && cargo clippy -- -D warnings && cargo build && cargo test

# PR 作成後の CI 監視
gh pr checks {pr-number} --watch
```

## Conventions

詳細は `rules/` 参照。

- **Git**: `rules/workflow.md` — ブランチ命名、コミット形式、PR フロー
- **CI**: `rules/ci.md` — CI パイプライン、失敗時対応
- **Code Style**: `rules/code-style.md` — Rust コーディング規約

## Key Dependencies

| Crate | 用途 |
|-------|------|
| `cargo_metadata` | Cargo メタデータ取得 (workspace_members 等) |
| `petgraph` | 依存グラフ構造 (StableGraph) |
| `anyhow` | エラーハンドリング |
| `serde` + `toml` | ルールファイル解析 |
| `colored` | 違反パッケージの赤字表示 |

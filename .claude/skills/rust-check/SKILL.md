---
name: rust-check
description: CI と同等のローカル検証を一括実行する
---

# /rust-check

CI パイプラインと同等のチェックをローカルで実行する。
PR 作成前に必ず実行すること。

## Steps

1. `cargo fmt --check` — フォーマット確認
2. `cargo clippy -- -D warnings` — lint (全警告エラー)
3. `cargo build` — コンパイル確認
4. `cargo test` — テスト実行

## Instructions

上記 4 コマンドを順番に実行してください。
いずれかが失敗した場合は、原因を特定して修正してください。
全て成功したら結果サマリを報告してください。

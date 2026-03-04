# Rust コーディング規約

## エディション & ツールチェイン

- Rust edition **2024**
- 最新 stable を使用 (`rustup update stable` で更新)
- `rustfmt.toml` / `clippy.toml` なし (すべてデフォルト設定)

## フォーマット & Lint

- `cargo fmt` — デフォルト設定
- `cargo clippy -- -D warnings` — 全警告をエラー扱い
- CI では `RUSTFLAGS: -D warnings` も設定されている

## エラーハンドリング

- `anyhow::Result` / `anyhow::Error` を使用
- `anyhow!()` マクロでエラーメッセージ生成
- `.context()` / `.with_context()` でエラーに文脈を付与

## テスト

- ユニットテストはモジュール内 `#[cfg(test)] mod tests { ... }` に配置
- 統合テスト用のデモ crate が `tests/demo_crates/` にある
  - `clean-arch/`: 違反なし (SUCCESS ケース)
  - `tangled-clean-arch/`: 違反あり (FAILURE ケース)
- テスト関数名: `test_{feature}_{scenario}` (例: `test_handler_success`)
- 遅いテストには `#[ignore]` を付与

## モジュール構成

- `mod.rs` パターンでモジュールを構成
- `pub(crate)` で内部モジュールの可視性を制限
- 構造体は derive マクロを活用 (`Debug`, `Clone`, `Default`)

## format 文字列

- `format!("{}", var)` ではなく `format!("{var}")` を使う (clippy::uninlined_format_args)
- `anyhow!()` マクロ内も同様

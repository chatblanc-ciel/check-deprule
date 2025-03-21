# check-dependency-rule

English version is [here](./docs/README_en.md).

## 使い方
### `dependency_rules.toml`

下記のようなcrate間の依存ルールを記載したファイルを作成する。

```yaml
[[rules.rule]]
package = "core"
forbidden_dependencies = ["handler"]

[[rules.rule]]
package = "interactor"
forbidden_dependencies = [
    "database",
    "handler",
]
```

この時`core`packageは`handler`をdenpendenciesに
もってはいけないというルールを定義している。

### command

CLIアプリケーションのインストール

```bash
cargo install check-deprule
```

ルールを満たさないパッケージが赤字で示されます。

```bash
check-deprule
```

## remaining tasks

- ルール定義ファイルの指定
- ルールをパッケージ名だけではなく、柔軟に記載できるようにする
- clapを使ったCLIアプリケーション化
- 違反パッケージの特定とdepenndency treeの出力を分ける

# Special Thanks
- [cargo-tree](https://github.com/sfackler/cargo-tree/tree/master)
- [cargo tree(std)](https://doc.rust-lang.org/nightly/nightly-rustc/cargo/ops/tree/index.html)
- [go-depcheck](https://github.com/v-standard/go-depcheck/tree/main)

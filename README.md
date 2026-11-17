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

この時`core`packageは`handler`をdependenciesに
もってはいけないというルールを定義している。

### command

CLIアプリケーションのインストール

```bash
cargo install check-deprule
```

ルールに違反する依存を 1 行ずつ出し、最後に件数を出します。違反があれば終了コードは 1、無ければ 0 です。

```bash
check-deprule
```

```text
violation: core -> handler
1 violation found
```

依存の木の全体を見るときは `--tree` を付けます。木の中の違反の行には `[violation]` が付きます (端末では赤字にもなります)。木は workspace の大きさに応じて大きくなります。

```bash
check-deprule --tree
```

## Roadmap

- ルールをパッケージ名だけではなく、柔軟に記載できるようにする

# Special Thanks
- [cargo-tree](https://github.com/sfackler/cargo-tree/tree/master)
- [cargo tree(std)](https://doc.rust-lang.org/nightly/nightly-rustc/cargo/ops/tree/index.html)
- [go-depcheck](https://github.com/v-standard/go-depcheck/tree/main)

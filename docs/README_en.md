# check-dependency-rule
## Usage
### `dependency_rules.toml`

Create a file named `dependency_rules.toml` that describes the dependency rules between crates, as shown below:

```toml
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

This example defines a rule that the `core` package is not allowed to have `handler` in its dependencies.  Similarly, `interactor` cannot depend on `database` or `handler`. (The original Japanese phrasing is slightly more explicit; I've made it flow better in English while preserving the meaning.)

### Command

Install the CLI application:

```bash
cargo install check-deprule
```

Run the tool. Packages that violate the rules will be highlighted in red:

```bash
check-deprule
```

## Remaining Tasks

-   Allow specifying the rule definition file. (Instead of assuming `dependency_rules.toml`)
-   Make the rule definitions more flexible, not just limited to package names.
-   Convert to a CLI application using `clap`.  (This essentially means to improve the argument parsing and user interface.)
- Separate the identification of violating packages from the output of the dependency tree. (This is about improving the output and making it clearer *why* a package is in violation.)

# Special Thanks
- [cargo-tree](https://github.com/sfackler/cargo-tree/tree/master)
- [cargo tree(std)](https://doc.rust-lang.org/nightly/nightly-rustc/cargo/ops/tree/index.html)
- [go-depcheck](https://github.com/v-standard/go-depcheck/tree/main)

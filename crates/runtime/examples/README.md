# clap-layers examples

Run these from the **workspace root**, which is the working directory `cargo run`
uses — the `file = "examples/config.toml"` paths in the examples resolve from there.

| Example | What it shows |
|---------|---------------|
| [`cli_only.rs`](cli_only.rs) | The simplest case: flags and defaults, no other layers |
| [`environment_vars.rs`](environment_vars.rs) | The environment layer and how variables are named |
| [`config_file.rs`](config_file.rs) | The TOML file layer, and that a missing file is not an error |
| [`precedence_demo.rs`](precedence_demo.rs) | All four layers, including a flag that equals the default |
| [`sensitive_data.rs`](sensitive_data.rs) | `no_env` / `no_file`, keeping a password off every other layer |
| [`dynamic_values.rs`](dynamic_values.rs) | `no_cli`, for fields that must never be a flag |
| [`complete_example.rs`](complete_example.rs) | Everything together, with error reporting |

```bash
cargo run --example precedence_demo
MYAPP_PORT=8080 cargo run --example environment_vars
```

## Environment variables

Variables are `PREFIX_FIELD`, **uppercased**: with `env_prefix = "MYAPP"`, the field
`port` reads `MYAPP_PORT` and `db_password` reads `MYAPP_DB_PASSWORD`.

The environment layer is only active when `env_prefix` is set — without a prefix a
field named `path` would read the ambient `PATH`.

Values are parsed as TOML values, so `MYAPP_TAGS='["a","b"]'` fills a `Vec<String>`.
Anything that is not valid TOML is taken as a plain string, so `MYAPP_HOST=localhost`
needs no quoting.

## Precedence

Highest wins:

1. **An explicitly typed CLI flag** — `--port 8080`
2. **An environment variable** — `MYAPP_PORT=8080`
3. **The config file** — `port = 8080`
4. **The built-in default** — `default_value_t = 3000`

A flag you typed beats the lower layers *even when the value you typed is the same as
the default*. `precedence_demo` demonstrates exactly that case.

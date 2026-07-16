# Design & Rationale

This document explains **why `clap-layers` exists, what it must get right, and how it is meant
to work internally**. It's aimed at contributors and anyone curious about the design decisions.
For usage, see the [README](../README.md); for the working agreement (build/test/lint loop,
coverage, API rules), see [CLAUDE.md](../CLAUDE.md).

## Motivation

[clap](https://crates.io/crates/clap) is the de-facto CLI parser for Rust, but it deliberately
does **not** handle layered configuration — merging values from CLI flags, environment
variables, and a config file with sensible precedence. clap's maintainer keeps this out of core
on purpose and has invited the ecosystem to build it on top and see what works.

The demand has been real and unresolved for years: a long-running clap discussion collected many
subscribers asking for exactly this, with no single "blessed" solution emerging. Several crates
have attempted it, and each stumbles on at least one of the correctness traps below. clap even
merged a documentation example wiring config manually — a docs page standing in for a crate that
should exist.

So the opportunity here is **not novelty — it's quality**. This crate wins by being *correct*,
by producing *excellent errors*, and by being *well documented*, not by inventing a new concept.
That framing drives every requirement that follows.

## What it must get right (the five requirements)

These are the acceptance criteria. They're the reasons the crate exists; the test suite must
prove each one, and no change may regress them.

1. **Explicit-vs-default detection.** A value in the config file must override a clap *default*,
   but must lose to a flag the user *explicitly passed on the command line* — even when the
   passed value happens to equal the default. This is the core correctness claim. It relies on
   clap's `ArgMatches::value_source()` to tell "the user typed this" apart from "this is the
   default," and that distinction must be plumbed through the merge, hidden behind the derive.

2. **`--help` must keep showing real defaults.** A tempting-but-wrong implementation wraps every
   field in `Option<T>` to detect whether it was set. That breaks `--help`, which then shows
   every field as optional with no default. Fields must keep their native clap
   `default_value_t`, so help still prints `[default: 3000]`. Presence is detected via
   `value_source()`, never by making fields optional.

3. **One struct, no duplication.** The user writes a **single** struct. Its fields and
   doc-comments drive clap parsing, `serde` deserialization, and environment reading all at once.
   The user must never have to hand-maintain a parallel "partial" or shadow struct. Generating
   such a struct *inside* the macro would be acceptable, as long as it stayed invisible; as
   built, the derive doesn't need one — see [How the derive works](#how-the-derive-works).

4. **Source-attributed errors.** When a value is invalid, the error names the layer it came from,
   e.g. `invalid value 'foo' for 'port' — from config.toml, line 12`. A merge that collapses all
   sources first and only then reports a generic failure — leaving the user unable to tell which
   file or variable was at fault — is exactly the failure mode we're reacting against.

5. **Field-level control.** Merge behavior is often field-specific, so it can't be a single
   top-level policy. Support per-field merge strategies (e.g. replace vs. append) and per-field
   layer markers (`no_cli` / `no_file` / `no_env`) for values that shouldn't exist in every
   layer. This directly answers long-standing feedback that a one-size-fits-all merge is wrong.

## Additional requirements

Beyond the five, these round out a credible v1-track design:

- **`--dump-config`**: print the effective, merged configuration. Repeatedly requested by users.
- **Config-file discovery**: an explicit path flag, plus implicit discovery (walk up from CWD,
  and XDG locations), plus an `--isolated` escape hatch that disables discovery entirely.
- **Format-agnostic, without bloat**: TOML is built in by default; any `serde::Deserialize`
  format is pluggable. We deliberately do **not** bundle every format crate behind a thicket of
  cargo features — that "feature soup" is a named criticism of an existing competitor.

## The API, as built

This is the v0.1 API. The example is compiled as a doctest, so it cannot drift away from the
real crate — an unchecked example is how this document came to describe an API that never
shipped.

```rust,no_run
use clap::Parser;
use clap_layers::Layered;

#[derive(Parser, Layered, Debug)]
#[command(version, about)]
#[layered(file = "myapp.toml", env_prefix = "MYAPP")]
struct Config {
    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Verbosity
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Config/env only — never a CLI flag.
    ///
    /// `#[arg(skip)]` is what actually keeps clap from defining a flag; a separate derive
    /// cannot remove an argument clap has already registered. The derive rejects `no_cli`
    /// without it rather than let the field stay exposed, and `skip = <expr>` supplies the
    /// built-in default that `default_value_t` would give a normal flag.
    #[layered(no_cli)]
    #[arg(skip = 5u32)]
    retry_budget: u32,
}

fn main() {
    // `layered()` handles CLI errors and `--help` itself, exactly as clap's `parse()` does.
    // Print the Display form of anything else: `?` in `main` prints the *Debug*
    // representation, which throws away the source attribution requirement 4 exists for.
    let cfg = Config::layered().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    });

    println!("{cfg:?}"); // flag > env > file > default
}
```

`layered()` reads the real process arguments and environment. Its counterpart takes both
explicitly, which is what makes layered configuration testable and safe to run in parallel:

```rust
# use clap::Parser;
# use clap_layers::{Env, Layered};
# #[derive(Parser, Layered, Debug)]
# #[layered(env_prefix = "MYAPP")]
# struct Config {
#     #[arg(long, default_value_t = 3000)]
#     port: u16,
# }
let env = Env::from_iter([("MYAPP_PORT", "8080")]);

assert_eq!(Config::layered_from(["myapp"], &env)?.port, 8080);
assert_eq!(Config::layered_from(["myapp", "--port", "3000"], &env)?.port, 3000);
# Ok::<(), clap_layers::LayeredError>(())
```

Features still on the roadmap — per-field `merge` strategies and `discover` — are **not**
implemented, and the derive rejects them as unknown options rather than ignoring them. Their
intended shape is sketched below, and is not valid code today:

```rust,ignore
// v0.2 — NOT yet implemented.
#[layered(file = "myapp.toml", env_prefix = "MYAPP", discover = "xdg")]
struct Config {
    /// CLI replaces; config appends
    #[arg(long)]
    #[layered(merge = "append")]
    include_paths: Vec<std::path::PathBuf>,
}
```

## How the derive works

The expansion is deliberately thin. All merge logic lives in the runtime crate, so it is
compiled and tested once rather than re-emitted into every field of every user's struct.

For each `#[derive(Layered)]` struct, the generated `layered_from`:

1. Builds clap's `Command` and parses the arguments into `ArgMatches`. The user's
   `#[derive(Parser)]` is untouched, which is what keeps requirement 2 — `--help` still
   renders the real `default_value_t`.
2. Reads and parses the TOML file **once**, retaining each value's source span so a bad value
   can name its line (requirement 4).
3. Resolves each field through `__private::resolve`, which walks the layers in order and stops
   at the first that supplies a value.

There is **no partial or shadow struct**. An earlier sketch of this design proposed mirroring
the struct in all-`Option` form and merging partials; it turned out to be unnecessary. Reading
`ArgMatches::value_source()` per field answers "did the user type this?" directly, and `serde`
decodes each field from its layer independently, so nothing needs an intermediate
representation. That also means requirement 3 costs nothing: there is no shadow struct to keep
invisible.

The precedence rule each field follows:

| `value_source()` | Meaning | Outcome |
| ---------------- | ------- | ------- |
| `CommandLine`    | the user typed the flag | use it; consult no lower layer |
| `EnvVariable`    | clap's own `#[arg(env)]` found it | use it; it outranks the file |
| `DefaultValue`   | clap filled in `default_value_t` | try env, then file, then keep the default |
| `None`           | not a clap argument at all (`#[arg(skip)]`) | try env, then file, then keep `Default::default()` |

Two details matter more than they look:

- **`EnvVariable` counts as explicit.** A value clap read from the environment is a real
  user-supplied value, not a default, so it must beat the config file. This is what lets clap's
  native `#[arg(env = "...")]` compose with our layers instead of fighting them.
- **`value_source()` panics in debug builds for an unknown id**, so it must never be called for
  a field clap does not define — `#[arg(skip)]`, `#[command(flatten)]` and subcommand fields are
  detected at expansion time and never queried.

Keep the runtime crate near-zero-dependency; the proc-macro crate leans only on `syn` / `quote` /
`proc-macro2`. The runtime depends on `clap`, plus `toml` and `serde` for the file layer. See
[CLAUDE.md](../CLAUDE.md) for the workspace split.

## Consequences of the design

Requirements 2 and 4 pull against convenience in a few places. These trade-offs are deliberate,
and each is pinned by a test so it cannot change silently.

- **A required field cannot be filled from a lower layer.** clap enforces required-ness while
  parsing, before any layer is consulted, so a field with no default and no `Option<T>` fails on
  the command line even when the config file sets it. The fix would be to inject file/env values
  into clap as defaults before parsing — but then `--help` would advertise config-file values as
  defaults (breaking requirement 2) and clap would report bad values with no idea which layer
  they came from (breaking requirement 4). Giving the field a `default_value_t` or making it
  `Option<T>` costs the user far less than either regression.
- **Unknown config-file keys are ignored, not rejected.** Fields are read from the parsed table
  by name, so a typo silently does nothing. Rejecting unknown keys would break forward
  compatibility for a config file shared across versions or tools; a strict opt-in is a better
  answer than a default, and is deferred to v0.2.
- **Environment sequences must be TOML arrays** (`MYAPP_TAGS='["a","b"]'`). Environment values
  are parsed as TOML values so that `Vec<T>`, `Option<T>` and nested tables work at all; clap's
  `value_delimiter` is a command-line concern and does not apply. A value that isn't valid TOML
  falls back to a plain string, so `MYAPP_HOST=localhost` needs no quoting.

## Non-goals (for now)

- **Subcommands are out of scope for v0.1.** The CLI wants an enum (one subcommand per run) while
  a config file wants a struct (settings for all subcommands); reconciling the two is the hardest
  design problem and is deliberately deferred. The attribute grammar should be designed so this
  can be added later without breaking changes, but it is not built yet.

## Roadmap

- **v0.1** *(implemented)* — the derive macro; `flag > env > file > default` precedence with
  correct `ValueSource` semantics; TOML file source; env source with prefix; source-attributed
  errors; `no_cli` / `no_file` / `no_env` markers; the precedence-matrix test suite.
- **v0.2** — per-field merge strategies; `--dump-config`; config-file discovery + `--isolated`;
  a pluggable-format example (e.g. JSON).
- **v0.3** — subcommand support (the enum ↔ struct mapping).

## Prior art & positioning

Several crates occupy adjacent space. This is meant as a fair map, not a takedown — the point is
to be clear about where `clap-layers` aims to differ.

| Crate              | clap-aware | Explicit-vs-default handled | Source-attributed errors |
| ------------------ | ---------- | --------------------------- | ------------------------ |
| **clap-layers**    | yes        | yes                         | yes                      |
| twelf              | yes        | partial                     | no                       |
| confique           | no         | n/a                         | yes                      |
| figment            | no (engine) | n/a                        | yes                      |
| clap-config-file   | yes        | partial                     | partial                  |

The `clap-layers` row reflects behaviour covered by the test suite; the others are a
point-in-time reading of each crate's documentation. `figment` is a general layering engine but
isn't clap-aware. `confique` is well-maintained but
has no clap integration. The clap-integrated options each miss at least one of the five
requirements above — most commonly explicit-vs-default detection and error attribution. That gap
is the whole reason to build this.

## References

- clap discussion, "Designing for layered configs":
  <https://github.com/clap-rs/clap/discussions/2763>
- clap issue #3113 (same topic): <https://github.com/clap-rs/clap/issues/3113>
- clap's manual-figment docs example (PR #6162):
  <https://github.com/clap-rs/clap/pull/6162>
- Related clap issues: #1695 (help defaults), #2683 (typed `ArgMatches`), #748, #1206

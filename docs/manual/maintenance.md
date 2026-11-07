# Building and testing the manual

The book source is `docs/SUMMARY.md`, `docs/introduction.md`, and `docs/manual/`,
with configuration in
`book.toml`. Images reuse the repository's `docs/images/` assets. Build output
goes under the ignored `target/book/` directory.

Install [mdBook](https://rust-lang.github.io/mdBook/) using its installation
instructions, then run from the repository root:

```sh
mdbook build
mdbook serve --open
```

`serve` rebuilds on edits and hosts a local preview. It does not publish the book.
The static `target/book/index.html` is also usable locally.

## Check executable examples

```sh
tools/check-book.sh
```

The script builds the book, checks local links and image references, builds the
library with `serde`, then runs mdBook's
Rust doctests against that build. Examples use assertions for observable
behaviour. File-writing examples use `no_run` so checking the manual does not
write application output. The isolated Cargo target directory avoids mixing
incompatible feature builds when rustdoc resolves external crates.

Run the repository's normal checks when changing library code:

```sh
tools/ci/check.sh
tools/ci/js-guard.sh
```

The ordinary checks use checked-in fixtures and do not need Node. Development
oracle/regeneration commands are separate and are documented in `UPSTREAM.md`.
Changing the manual does not regenerate built-in icon tables or publish a crate.

Keep examples aligned with the chosen source revision. Update behaviour and
instructions in place, and keep release history in the changelog. API details
belong in rustdoc; this manual focuses on application workflows, relationships
between APIs, and practical limitations.

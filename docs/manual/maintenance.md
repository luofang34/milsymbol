# Building and testing the manual

The book source is `docs/SUMMARY.md`, `docs/introduction.md`, and `docs/manual/`,
with configuration in
`book.toml`. Images reuse `docs/images/` assets and SVGs rendered from compiled examples.
The symbol lab calls the same library through `tools/book-wasm/`; its JavaScript
only connects controls and displays the Rust output. Build output
goes under the ignored `target/book/` directory.

The [published manual](https://luofang34.github.io/milsymbol/) is hosted on
GitHub Pages. The `Usage manual` workflow checks the book on pull requests and
publishes a checked build on pushes to `main`. Its manual dispatch can republish
`main`. The repository's Pages source must be **GitHub Actions**.

Install [mdBook](https://rust-lang.github.io/mdBook/), Python 3.11 or newer,
and Node for the browser-boundary regression check. Add the WASM target and
install the bindings generator matching `tools/book-wasm/Cargo.toml`:

```sh
rustup target add wasm32-unknown-unknown
version=$(python3 -c 'import tomllib; print(tomllib.load(open("tools/book-wasm/Cargo.toml", "rb"))["dependencies"]["wasm-bindgen"].removeprefix("="))')
cargo install --locked wasm-bindgen-cli --version "$version"
tools/build-book.sh
python3 -m http.server --bind 127.0.0.1 --directory target/book 8000
```

Open `http://127.0.0.1:8000/`. Re-run the build script when changing Rust,
examples, or book source. The symbol lab requires HTTP because browsers load
its WASM module as an asset.

## Check executable examples

```sh
tools/check-book.sh
```

The script renders example SVGs, builds the WASM module, checks local links,
executes the shipped WASM artifact, and runs the browser protocol tests and
mdBook's Rust doctests. Snippets in [worked examples](examples.md) include
anchored regions of compiled Rust files; their images come from executing those
same functions. Changes to API usage or output are checked during every book
build. Explanatory prose still needs review when behaviour changes.

File-writing examples use `no_run` so checking the manual does not write
application output. The isolated Cargo target directory avoids mixing
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

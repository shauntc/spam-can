# spam-can

A toolbox for spamming requests at an api and analyzing the results. Includes 3 tools:

- `spam` - for collecting data which is then stored in [rkyv](https://github.com/rkyv/rkyv) files
  - `spam.toml` - config file listing the requests in [toml](https://toml.io/en/) (can also be a `.json` file),
- `plot` - loads the data and creates histograms for it
- `extract` - for finding requests in a certain time range

## Dependencies

- [rust](https://www.rust-lang.org/) (installing rust also installs `cargo`, the rust package manager)

## Usage

Run each tool using cargo with the `--bin` parameter to select the tool

- eg `cargo run --bin spam -- -p 30`, sections of this call are:
  1. `cargo run` - build and run a rust program, can also be replaced with `cargo run --release` to run the release build (though it doesn't have much impact for this tool). The first build will likely take a while but all crates (the name for packages in the rust ecosystem) are cached after that
  2. `--bin spam` - `--bin` selects the binary and corresponds to the files in the `src/bin` directory, each tool is a separate binary (`spam`, `plot`, and `extract`)
  3. `-- -p30` - parameters for the binary, `-p 30` in this case. This can be replaced with `-- --help` to list all available options (`-p` here is for parallelism)

### Steps

1. configure a `.toml` or `.json` file for `spam` or edit `spam.toml` (the default config), configuration parameters can be found in `src/config.rs`, the root is `SpamConfig`
2. run `spam` specifying your `.toml` file with `--config-path example.toml` (defaults to `spam.toml`)
   - eg. `cargo run --bin spam -- --config-path example.toml -p 30`
3. wait for requests to complete
4. run `plot` to create graphs
5. view graphs in `out/graphs` (or at the location you specified with `--output-dir example/dir`)
6. _optional_ run `extract` with `--min-ms` and `--max-ms` to get the data for a request in that time range

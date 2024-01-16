# spam-can

A toolbox for spamming requests at an api and analyzing the results. Includes 3 commands:

- `spam spam` - for collecting data which is then stored in [rkyv](https://github.com/rkyv/rkyv) files
  - `spam.toml` - config file listing the requests in [toml](https://toml.io/en/) (can also be a `.json` file),
- `spam plot` - loads the data and creates histograms for it
- `spam extract` - for data analysis

  - `percentiles` - show the p75/95/99, etc
  - `range` - print a number of requests in a range
  - `failures` - print some number of failures

use `--help` to get the full list of parameters for each binary (or subcommand) (eg. `spam extract range --help`)

### Dependencies:

- [rust](https://www.rust-lang.org/tools/install)
  - more info about [rust at microsoft](https://aka.ms/rust)
- [ms-stable](https://eng.ms/docs/more/rust/services/rust-toolchain) version of the rust toolchain (microsoft internally built version)
- feed authentication for [ContentServices_PublicPackages](https://msasg.visualstudio.com/ContentServices/_artifacts/feed/ContentServices_PublicPackages/connect)
  1. follow the above link and select `cargo`
  2. follow one of the methods under "Log into the registry with the following command". "With AzureCLI" is recommended:
  ```
  az login
  az account get-access-token --query "join(' ', ['Bearer', accessToken])" --output tsv | cargo login --registry ContentServices_PublicPackages
  ```

## Usage

Run each tool using cargo with a command parameter to select the tool

- eg `cargo run -- extract failures -c 10`, sections of this call are:
  1. `cargo run` - build and run a rust program, can also be replaced with `cargo run --release` to run the release build (though it doesn't have much impact for this tool). The first build will likely take a while but all crates (the name for packages in the rust ecosystem) are cached after that
  2. `--` separates the built binary's parameters from `cargo`'s
  3. `extract` the command to be used (`spam`, `plot` or `extract`)
  4. `failures` a sub command specific to the `extract` command (to see subcommands append `--help` to any of the commands)
  5. `-c 10` - parameters for the binary, `-c 10` in this case. This can be replaced with `--help` to list all available options (`-c` here is for count)

alternately you can install the tools locally using `cargo install --path .` (while in this folder) which will put the `spam` binary in the cargo bin folder (`$HOME/.cargo/bin` by default) which `rustup` will have added to your path and you can run each tool from there eg. `spam spam` or `spam extract percentiles` or `spam plot`

### Steps

1. configure a `.toml` or `.json` file for `spam` or edit `spam.toml` (the default config), configuration parameters can be found in `src/config.rs`, the root is `SpamConfig`
2. run `spam spam` specifying your `.toml` file with `--config-path example.toml` (defaults to `spam.toml`)
   - eg. `cargo run -- --config-path example.toml spam`
3. wait for requests to complete
4. run `spam plot` to create graphs
5. view graphs in `out/graphs` (or at the location you specified with `--output-dir example/dir`)
6. _optional_ run `spam extract range` with `--min-ms` and `--max-ms` to get the data for a request in that time range
7. _optional_ run `spam extract percentiles` to get a set of percentiles for each test
8. _optional_ run `spam extract failures -c 10` to list the data from `-c` failures

appending `-h` or `--help` to any command will list the available options, eg. `spam extract percentiles --help`

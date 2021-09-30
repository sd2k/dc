`dc`

A tiny docker-compose wrapper which searches parent directories for the docker-compose.yaml file, and passes it to the `-f` flag of docker-compose.

# Installation

Install a recent-ish Rust toolchain if you've not already got one. The recommended method is using [Rustup](https://rustup.rs/).

Then run `cargo install --git https://github.com/sd2k/dc` from the repository root. This will place the `dc` binary in `~/.cargo/bin`. You may need to modify your PATH to move that directory to be earlier than `/usr/bin`, since `dc` already exists on Unix as a calculator.

## Shell completion

Shell completion is available for zsh. Installation instructions for oh-my-zsh are in completion/zsh/README.md.


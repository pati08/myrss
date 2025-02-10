A simple chat feed with whatever stuff I felt like adding

# How to setup and run
## Building
Build with nix:
- `nix build` - build for x86_64
- `nix build .#docker` - build docker image for x86_64
- `nix build .#myrss-arm` - build for arm
- `nix build .#docker-arm` - build docker image for arm platform

Build with cargo: `cargo build --release`. If building with cargo, you can include a `Secrets.toml` file at the root to set environment variable secrets for running, specified in the next section. You should always keep this `.gitignore`-d. This is not currently supported when building with nix.

## Running

### Ports
The server listens on port 3000 unless another is selected through the environment variable. It is publicly exposed to the network by default, but this can be disabled by setting `RSS_DO_NOT_PUBLISH=1`

## Environment
There are environment variables with default values used to control behavior. The only required one is `GROQ_API_KEY`, which can also be provided in `Secrets.toml` at build time to encode them as strings in the binary instead.

The following optional environment variables are also supported:

Name|Value|Description
--- | --- | ----------
`SERVER_PORT` | `unsigned_int` | port number to listen on 
`RSS_DO_NOT_PUBLISH` | values other than `1` have no effect | whether to run the server on `127.0.0.1` instead of `0.0.0.0`
`AI_MAX_HISTORY_CHARS` | `unsigned_int` | maximum number of characters before cutting off messages in AI context
`BOT_SAVE_PATH` | `path` | path to save and read bot data from

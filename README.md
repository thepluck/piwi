# Piwi 🧂⛏️

Piwi is a CLI for quickly finding salts that create flags matching Uniswap V4 Hooks' addresses via CREATE2 or CREATE3.
Written in Rust with [Alloy](https://github.com/alloy-rs/core).

Piwi is heavely inspired by [Maldon](https://github.com/flood-protocol/maldon), with the difference that it focuses on Uniswap V4 Hooks.

## Installation

```bash
git clone https://github.com/thepluck/Piwi.git
cd Piwi
# Run it directly
cargo run --release -- create2 --factory <FACTORY> <CALLER> <INIT_CODE_HASH> <FLAGS>

# Add it to your path
cargo install --path .
```

## Usage

```
Usage: piwi <COMMAND>

Commands:
  create2  Mines a CREATE2 salt
  create3  Mines a CREATE3 salt
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help (see a summary with '-h')

Usage: piwi create2 [OPTIONS] <DEPLOYER> <INIT_CODE_HASH> <FLAGS>

Arguments:
  <DEPLOYER>        Address of the contract deployer
  <INIT_CODE_HASH>  Hash of the initialization code
  <FLAGS>           Hex string representing the desired flags

Options:
  -f, --factory <FACTORY>   Address of the Factory contract. Defaults to the Archanid's Factory
  -p, --prefix <PREFIX>     Optional prefix for the mined address. Defaults to an empty string
  -h, --help                Print help (see a summary with '-h')

Usage: piwi create3 [OPTIONS] <DEPLOYER> <FLAGS>

Arguments:
  <DEPLOYER>  Address of the contract deployer
  <FLAGS>     Hex string representing the desired flags

Options:
  -f, --factory <FACTORY>   Address of the Factory contract. Defaults to the LayerZero's Factory
  -p, --prefix <PREFIX>     Optional prefix for the mined address. Defaults to an empty string
  -h, --help                Print help (see a summary with '-h')
```

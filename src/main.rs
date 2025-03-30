mod cli;
mod mine;

use alloy_primitives::{Address, address};
use clap::Parser;
use {
    cli::Piwi,
    mine::{Create2Miner, Create3Miner, Miner},
};

/// The standard CREATE2 factory address on Ethereum
/// See: https://github.com/Arachnid/deterministic-deployment-proxy
const CREATE2_DEFAULT_FACTORY: Address = address!("0x4e59b44847b379578588920cA78FbF26c0B4956C");

/// The standard CREATE3 factory address on Ethereum
/// See: https://www.npmjs.com/package/@layerzerolabs/create3-factory
const CREATE3_DEFAULT_FACTORY: Address = address!("0x8Cad6A96B0a287e29bA719257d0eF431Ea6D888B");

/// Entry point for the Piwi smart contract address mining tool.
///
/// This application allows users to "mine" for vanity addresses for smart
/// contracts by finding salt values that produce desirable contract addresses
/// when used with either CREATE2 or CREATE3 deployment mechanisms.
///
/// # Error
///
/// Returns a hex parsing error if any hex inputs are malformed.
fn main() {
    let (address, salt) = match Piwi::parse() {
        Piwi::Create2 {
            deployer,
            factory,
            init_code_hash,
            flags,
            prefix,
        } => {
            // Use the provided factory or fall back to the default CREATE2 factory
            let factory = factory.unwrap_or(CREATE2_DEFAULT_FACTORY);

            // Use the provided prefix or fall back to an empty string
            let prefix = prefix.unwrap_or_default();

            // Mine for an address matching the flags using CREATE2 deployment
            Create2Miner::new(deployer, factory, init_code_hash).mine(&flags, &prefix)
        }
        Piwi::Create3 {
            deployer,
            factory,
            flags,
            prefix,
        } => {
            // Use the provided factory or fall back to the default CREATE3 factory
            let factory = factory.unwrap_or(CREATE3_DEFAULT_FACTORY);

            // Use the provided prefix or fall back to an empty string
            let prefix = prefix.unwrap_or_default();

            // Mine for an address matching the flags using CREATE3 deployment
            Create3Miner::new(deployer, factory).mine(&flags, &prefix)
        }
    };

    // Output the discovered salt and resulting contract address
    println!("Found salt {salt:?} ==> {address:?}");
}

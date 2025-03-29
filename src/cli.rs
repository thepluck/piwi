use alloy_primitives::{Address, FixedBytes};

/// Command-line interface for the Piwi tool.
///
/// Piwi is a tool for mining CREATE2 and CREATE3 salts specifically optimized
/// for Uniswap V4 Hooks.
#[derive(Clone, Debug, clap::Parser)]
#[command(
    name = "piwi",
    about = "Piwi is a fast CREATE2 and CREATE3 salt miner for Uniswap V4 Hooks."
)]
pub(super) enum Piwi {
    /// Mines a CREATE2 salt.
    ///
    /// CREATE2 is an opcode in Ethereum that allows contracts to be deployed
    /// at predetermined addresses.
    Create2 {
        /// Address of the contract deployer.
        deployer: Address,

        /// Address of the Factory contract. Defaults to the Archanid's Factory.
        #[clap(short, long)]
        factory: Option<Address>,

        /// Hash of the initialization code.
        init_code_hash: FixedBytes<32>,

        /// Hex string representing the desired flags.
        flags: String,
    },

    /// Mines a CREATE3 salt.
    ///
    /// CREATE3 is a pattern built on top of CREATE2 that allows for
    /// deterministic deployments that are immune to the contract's
    /// initialization code.
    Create3 {
        /// Address of the contract deployer.
        deployer: Address,

        /// Address of the Factory contract. Defaults to the LayerZero's
        /// Factory.
        #[clap(short, long)]
        factory: Option<Address>,

        /// Hex string representing the desired flags.
        flags: String,
    },
}

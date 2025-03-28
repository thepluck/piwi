use std::{ops::Deref, str::FromStr};

use alloy_primitives::{Address, FixedBytes};

/// A structure representing a binary pattern (consisting of only 0s and 1s).
///
/// This is used to specify the pattern to search for when mining CREATE2 and CREATE3 salts.
/// The pattern is stored as a boxed string for memory efficiency.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct BinaryPattern(Box<str>);

impl Deref for BinaryPattern {
    type Target = str;

    /// Dereferences the BinaryPattern to access the underlying string.
    ///
    /// # Returns
    /// A reference to the underlying string.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Errors that can occur when parsing a binary pattern from a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum BinaryPatternError {
    /// The pattern exceeds the maximum allowed length of 160 characters.
    #[error("the pattern's length exceeds 160 characters")]
    InvalidPatternLength,

    /// The pattern contains a character that is not a binary digit (0 or 1).
    #[error("the pattern is not in binary format")]
    InvalidCharacter(char),
}

impl FromStr for BinaryPattern {
    type Err = BinaryPatternError;

    /// Attempts to parse a string into a BinaryPattern.
    ///
    /// # Arguments
    /// * `s` - The string to parse
    ///
    /// # Returns
    /// * `Ok(BinaryPattern)` - If the string is a valid binary pattern
    /// * `Err(BinaryPatternError)` - If the string is too long or contains non-binary characters
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 160 {
            return Err(BinaryPatternError::InvalidPatternLength);
        }

        for c in s.chars() {
            if c != '0' && c != '1' {
                return Err(BinaryPatternError::InvalidCharacter(c));
            }
        }

        Ok(Self(s.into()))
    }
}

/// Command-line interface for the Piwi tool.
///
/// Piwi is a tool for mining CREATE2 and CREATE3 salts specifically optimized for Uniswap V4 Hooks.
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

        /// Address of the Factory contract. Defaults to the Immutable CREATE2 Factory by 0age.
        #[clap(short, long)]
        factory: Option<Address>,

        /// Hash of the initialization code.
        init_code_hash: FixedBytes<32>,

        /// BinaryPattern to search for. Must be binary digits only and between 1 and 160 characters.
        parttern: BinaryPattern,
    },

    /// Mines a CREATE3 salt.
    ///
    /// CREATE3 is a pattern built on top of CREATE2 that allows for deterministic
    /// deployments that are immune to the contract's initialization code.
    Create3 {
        /// Address of the contract deployer.
        deployer: Address,

        /// Address of the Factory contract. Defaults to the LayerZero's Factory.
        #[clap(short, long)]
        factory: Option<Address>,

        /// BinaryPattern to search for. Must be binary digits only and between 1 and 160 characters.
        pattern: BinaryPattern,
    },
}

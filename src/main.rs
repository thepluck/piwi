mod cli;
mod mine;

use clap::Parser;

use alloy_primitives::{address, Address};

use {
    cli::Maldon,
    mine::{Create2Miner, Create3Miner, Miner},
};

const CREATE2_DEFAULT_FACTORY: Address = address!("0000000000ffe8b47b3e2130213b802212439497");

const CREATE3_DEFAULT_FACTORY: Address = address!("2dfcc7415d89af828cbef005f0d072d8b3f23183");

fn main() -> Result<(), hex::FromHexError> {
    let (address, salt) = match Maldon::parse() {
        Maldon::Create2 {
            deployer,
            factory,
            init_code_hash,
            pattern,
            tail_pattern,
        } => {
            let factory = factory.unwrap_or(CREATE2_DEFAULT_FACTORY);
            let tail: Option<&Vec<u8>>;
            let bytes;
            match tail_pattern {
                Some(tail_pattern) => {
                    bytes = tail_pattern.into_bytes()?;
                    tail = Some(&bytes);
                }
                None => {
                    tail = None;
                }
            }

            Create2Miner::new(factory, deployer, init_code_hash).mine(&pattern.into_bytes()?, tail)
        }
        Maldon::Create3 {
            deployer,
            factory,
            pattern,
            tail_pattern,
        } => {
            let factory = factory.unwrap_or(CREATE3_DEFAULT_FACTORY);
            let tail: Option<&Vec<u8>>;
            let bytes;
            match tail_pattern {
                Some(tail_pattern) => {
                    bytes = tail_pattern.into_bytes()?;
                    tail = Some(&bytes);
                }
                None => {
                    tail = None;
                }
            }

            Create3Miner::new(factory, deployer).mine(&pattern.into_bytes()?, tail)
        }
    };

    println!("Found salt {salt:?} ==> {address:?}");

    Ok(())
}

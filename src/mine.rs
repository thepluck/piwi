use alloy_primitives::{Address, FixedBytes, address, hex::FromHex, keccak256};
use rand::{Rng, rng};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

/// Maximum value for the nonce segment of the salt (6 bytes).
const MAX_NONCE: u64 = u64::MAX >> 16;

/// Bitmask that isolates the lower 14 bits of an Ethereum address.
const FLAGS_MASK: Address = address!("0x0000000000000000000000000000000000003fFF");

/// Converts a hex string to an Ethereum address.
///
/// # Arguments
/// * `hex` - The hex string to convert.
/// * `pad_leading_zeros` - If true, pads the hex string with leading zeros to
///   ensure it's 40 characters long, else pads with trailing zeros.
fn hex_to_address(hex: &String, pad_leading_zeros: bool) -> Address {
    // Pad the hex string with zeros to ensure it's 40 characters
    let padded_hex = if pad_leading_zeros {
        format!("{:0>40}", hex)
    } else {
        format!("{:0<40}", hex)
    };

    // Convert the padded hex string to address
    Address::from_hex(&padded_hex).expect("Could not convert hex string to address")
}

/// Computes a bitmask that isolates the upper `prefix_len` bits of an address.
fn compute_prefix_mask(prefix_len: usize) -> Address {
    let mask_number = if prefix_len % 2 == 0 {
        (1u64 << (prefix_len << 2)) - 1
    } else {
        (1u64 << ((prefix_len + 1) << 2)) - (15u64 << ((prefix_len - 1) << 2)) - 1
    };
    let mut mask_address = Address::default();
    mask_address[0..8].copy_from_slice(&mask_number.to_le_bytes());
    mask_address
}

/// Checks if a candidate address matches the specified flags and prefix.
///
/// # Arguments
/// * `flags` - The flags to match.
/// * `prefix` - The prefix to match.
/// * `prefix_mask` - The bitmask for the prefix.
/// * `candidate` - The candidate address to check.
fn check_candidate(
    flags: &Address,
    prefix: &Address,
    prefix_mask: &Address,
    candidate: &Address,
) -> bool {
    (candidate.bit_and(FLAGS_MASK) == *flags) && (candidate.bit_and(*prefix_mask) == *prefix)
}

/// Defines the interface for address mining algorithms.
///
/// Implementations must be thread-safe to enable parallel mining.
pub(super) trait Miner {
    /// Searches for a salt value that, when used for deployment, produces a
    /// contract address matching the specified pattern in its lower bits.
    ///
    /// The mining process:
    /// 1. Create a salt with the deployer address
    /// 2. Fill the middle section with random bytes
    /// 3. Systematically try different nonce values in the final section
    /// 4. Use parallel processing to speed up the search
    /// 5. Return the first matching address and its corresponding salt
    fn mine(&self, flags: &String, prefix: &String) -> (Address, FixedBytes<32>);
}

/// Implementation for mining vanity addresses using the CREATE2 deployment
/// method.
///
/// CREATE2 generates deterministic contract addresses based on:
/// - The factory contract address
/// - The deployer address
/// - The initialization code hash
/// - A 32-byte salt value
///
/// This allows finding a salt that produces a contract address with desired
/// properties.
///
/// The 32-byte salt used for mining is structured as follows:
/// - Bytes 0-19: Deployer address (prevents frontrunning by other users)
/// - Bytes 20-25: Random values (prevents collisions between mining sessions)
/// - Bytes 26-31: Nonce values (systematically explored during mining)
#[derive(Debug, Clone, Copy)]
pub(super) struct Create2Miner {
    /// Address of the account that will call the factory
    deployer: Address,
    /// Address of the factory contract that will perform the CREATE2 deployment
    factory: Address,
    /// Keccak256 hash of the contract's initialization bytecode
    init_code_hash: FixedBytes<32>,
}

impl Create2Miner {
    /// Creates a new CREATE2 miner with the specified parameters.
    ///
    /// # Arguments
    /// * `deployer` - The address that will call the factory contract
    /// * `factory` - The address of the CREATE2 factory contract
    /// * `init_code_hash` - The keccak256 hash of the contract initialization
    ///   code
    pub(super) fn new(deployer: Address, factory: Address, init_code_hash: FixedBytes<32>) -> Self {
        Self {
            deployer,
            factory,
            init_code_hash,
        }
    }
}

impl Miner for Create2Miner {
    fn mine(&self, flags: &String, prefix: &String) -> (Address, FixedBytes<32>) {
        // Convert the flags and prefix from hex strings to addresses
        let prefix_mask = compute_prefix_mask(prefix.len());
        let flags = hex_to_address(flags, true);
        let prefix = hex_to_address(prefix, false);

        // Create a random number generator
        let mut rng = rng();

        // Fill the first 20 bytes with the deployer address
        let mut salt_base = [0u8; 32];
        salt_base[0..20].copy_from_slice(self.deployer.as_slice());

        loop {
            // Fill the random segment (bytes 20-25) with new random values
            // for each batch of nonce attempts
            rng.fill(salt_base[20..26].as_mut());

            // Parallelize the search across different nonce values
            let mining_result = (0..MAX_NONCE).into_par_iter().find_map_any(move |nonce| {
                let mut salt = salt_base;

                // Set the nonce segment (bytes 26-31) with the current nonce value
                salt[26..32].copy_from_slice(&nonce.to_be_bytes()[2..]);

                // Calculate the resulting contract address
                let candidate = self.factory.create2(salt, self.init_code_hash);

                // Return the candidate if it matches the flags and prefix
                check_candidate(&flags, &prefix, &prefix_mask, &candidate)
                    .then(|| (candidate, FixedBytes::from_slice(&salt)))
            });

            // If we found a match, return it and exit
            if let Some(answer) = mining_result {
                break answer;
            }
            // Otherwise, try with a new set of random bytes
        }
    }
}

/// Implementation for mining vanity addresses using the CREATE3 deployment
/// method.
///
/// CREATE3 is a two-step deployment process:
/// 1. Deploy a proxy contract using CREATE2 with a salt
/// 2. The proxy then deploys the actual contract using CREATE
///
/// This provides address stability across different chains regardless of
/// the contract's initialization code.
///
/// The 32-byte salt used for mining is structured as follows:
/// - Bytes 0-19: Deployer address (prevents frontrunning by other users)
/// - Bytes 20-45: Random values (prevents collisions between mining sessions)
/// - Bytes 46-51: Nonce values (systematically explored during mining)
#[derive(Debug, Clone, Copy)]
pub(super) struct Create3Miner {
    /// Address of the account that will call the factory
    deployer: Address,
    /// Address of the factory contract that will perform the deployment
    factory: Address,
}

impl Create3Miner {
    /// Keccak256 hash of the CREATE3 proxy contract initialization code.
    /// This is a constant value used in the first step of CREATE3 deployment.
    const PROXY_INIT_CODE_HASH: [u8; 32] = [
        0x21, 0xc3, 0x5d, 0xbe, 0x1b, 0x34, 0x4a, 0x24, 0x88, 0xcf, 0x33, 0x21, 0xd6, 0xce, 0x54,
        0x2f, 0x8e, 0x9f, 0x30, 0x55, 0x44, 0xff, 0x09, 0xe4, 0x99, 0x3a, 0x62, 0x31, 0x9a, 0x49,
        0x7c, 0x1f,
    ];

    /// Creates a new CREATE3 miner with the specified parameters.
    pub fn new(deployer: Address, factory: Address) -> Self {
        Self { deployer, factory }
    }

    /// Computes the contract address that would result from deploying with the given salt.
    #[inline]
    fn compute_create3_address(&self, salt: &[u8; 52]) -> Address {
        use std::sync::atomic::{AtomicU64, Ordering};

        static ITERATION: AtomicU64 = AtomicU64::new(0);

        // Print the current iteration value for debugging
        let current_iteration = ITERATION.fetch_add(1, Ordering::Relaxed);
        if current_iteration % 1000000 == 0 {
            println!("iteration: {}", current_iteration);
        }

        // First deploy the proxy using CREATE2
        let proxy = self
            .factory
            .create2(keccak256(salt), Self::PROXY_INIT_CODE_HASH);

        // Then compute the address the proxy would deploy using CREATE
        proxy.create(0x1)
    }
}

impl Miner for Create3Miner {
    fn mine(&self, flags: &String, prefix: &String) -> (Address, FixedBytes<32>) {
        // Convert the flags and prefix from hex strings to addresses
        let prefix_mask = compute_prefix_mask(prefix.len());
        let flags = hex_to_address(flags, true);
        let prefix = hex_to_address(prefix, false);

        // Create a random number generator
        let mut rng = rng();

        // Fill the first 20 bytes with the deployer address
        let mut salt_base = [0u8; 52];
        salt_base[0..20].copy_from_slice(self.deployer.as_slice());

        loop {
            // Fill the random segment (bytes 20-25) with new random values
            // for each batch of nonce attempts
            rng.fill(salt_base[20..46].as_mut());

            // Parallelize the search across different nonce values
            let mining_result = (0..MAX_NONCE).into_par_iter().find_map_any(move |nonce| {
                let mut salt = salt_base;

                // Set the nonce segment (bytes 26-31) with the current nonce value
                salt[46..52].copy_from_slice(&nonce.to_be_bytes()[2..]);

                // Calculate the resulting contract address
                let candidate = self.compute_create3_address(&salt);

                // Return the candidate if it matches the flags and prefix
                check_candidate(&flags, &prefix, &prefix_mask, &candidate)
                    .then(|| (candidate, FixedBytes::from_slice(&salt[20..52])))
            });

            // If we found a match, return it and exit
            if let Some(answer) = mining_result {
                break answer;
            }
            // Otherwise, try with a new set of random bytes
        }
    }
}

#[test]
fn test_compute_create3_address() {
    use alloy_primitives::address;

    let deployer = address!("0x9fC3dc011b461664c835F2527fffb1169b3C213e");
    let factory = crate::CREATE3_DEFAULT_FACTORY;
    let miner = Create3Miner::new(deployer, factory);
    let mut salt = [2u8; 52];
    salt[0..20].copy_from_slice(deployer.as_slice());
    let computed = miner.compute_create3_address(&salt);
    assert_eq!(
        computed,
        address!("0x1298be70f771753b5490b4708513d9f0F513dd36")
    );
}

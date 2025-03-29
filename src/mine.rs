use alloy_primitives::{Address, FixedBytes, address, keccak256};
use rand::{Rng, rng};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};

/// Maximum value for the nonce segment of the salt (6 bytes).
/// We shift u64::MAX right by 16 bits (2 bytes) to get a 6-byte value.
const MAX_NONCE: u64 = u64::MAX >> 16;

/// Bitmask that isolates the bottom 14 bits of an Ethereum address.
/// Used to determine if an address matches the desired pattern.
const FLAGS_MASK: Address = Address(FixedBytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x3f, 0xff,
]));

/// Defines the interface for address mining algorithms.
///
/// Implementations must be thread-safe to enable parallel mining.
pub(super) trait Miner: Sync {
    /// Calculates the contract address that would result from deploying with the given salt.
    ///
    /// This is the core method that each miner must implement based on the
    /// deployment method (CREATE2, CREATE3, etc.).
    fn compute_address(&self, salt: &[u8; 32]) -> Address;

    /// Creates the initial salt value used for mining.
    fn generate_salt_base(&self) -> [u8; 32];

    /// Placeholder for any setup before the mining process begins.
    fn before_mine(&self) {}

    /// Searches for a salt value that, when used for deployment, produces a contract
    /// address matching the specified pattern in its lower bits.
    ///
    /// The mining process:
    /// 1. Creates a salt with the deployer address
    /// 2. Fills the middle section with random bytes
    /// 3. Systematically tries different nonce values in the final section
    /// 4. Uses parallel processing to speed up the search
    /// 5. Returns the first matching address and its corresponding salt
    fn mine(&self, flags: &Address) -> (Address, FixedBytes<32>) {
        // setup the mining process
        self.before_mine();

        let mut rng = rng();
        let mut salt_base = self.generate_salt_base();

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
                let candidate = self.compute_address(&salt);

                // Return the candidate if its lower bits match the pattern
                (candidate.bit_and(FLAGS_MASK) == *flags)
                    .then(|| (candidate, FixedBytes::from_slice(&salt)))
            });

            // If we found a match, return it and exit
            if let Some(found) = mining_result {
                break found;
            }
            // Otherwise, try with a new set of random bytes
        }
    }
}

/// Implementation for mining vanity addresses using the CREATE2 deployment method.
///
/// CREATE2 generates deterministic contract addresses based on:
/// - The factory contract address
/// - The deployer address
/// - The initialization code hash
/// - A 32-byte salt value
///
/// This allows finding a salt that produces a contract address with desired properties.
///
/// The 32-byte salt used for mining is structured as follows:
/// - Bytes 0-19: Deployer address (prevents frontrunning by other users)
/// - Bytes 20-25: Random values (prevents collisions between mining sessions)
/// - Bytes 26-31: Nonce values (systematically explored during mining)
///
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
    /// * `init_code_hash` - The keccak256 hash of the contract initialization code
    pub(super) fn new(deployer: Address, factory: Address, init_code_hash: FixedBytes<32>) -> Self {
        Self {
            deployer,
            factory,
            init_code_hash,
        }
    }
}

impl Miner for Create2Miner {
    fn compute_address(&self, salt: &[u8; 32]) -> Address {
        self.factory.create2(salt, self.init_code_hash)
    }

    fn generate_salt_base(&self) -> [u8; 32] {
        // Fills the first 20 bytes with the deployer address
        let mut salt = [0u8; 32];
        salt[0..20].copy_from_slice(self.deployer.as_slice());
        salt
    }
}

/// Implementation for mining vanity addresses using the CREATE3 deployment method.
///
/// CREATE3 is a two-step deployment process:
/// 1. Deploy a proxy contract using CREATE2 with a salt
/// 2. The proxy then deploys the actual contract using CREATE
///
/// This provides address stability across different chains regardless of
/// the contract's initialization code.
///
/// The 32-byte salt used for mining is structured as follows:
/// - Bytes 0-25: Random values (prevents collisions between mining sessions)
/// - Bytes 26-31: Nonce values (systematically explored during mining)
#[derive(Debug, Clone, Copy)]
pub(super) struct Create3Miner {
    /// Address of the account that will call the factory
    deployer: Address,
    /// Address of the factory contract that will perform the deployment
    factory: Address,
}

static mut SALT_BUFFER: [u8; 52] = [0u8; 52];

impl Create3Miner {
    /// Keccak256 hash of the CREATE3 proxy contract initialization code.
    /// This is a constant value used in the first step of CREATE3 deployment.
    const PROXY_INIT_CODE_HASH: [u8; 32] = [
        0x21, 0xc3, 0x5d, 0xbe, 0x1b, 0x34, 0x4a, 0x24, 0x88, 0xcf, 0x33, 0x21, 0xd6, 0xce, 0x54,
        0x2f, 0x8e, 0x9f, 0x30, 0x55, 0x44, 0xff, 0x09, 0xe4, 0x99, 0x3a, 0x62, 0x31, 0x9a, 0x49,
        0x7c, 0x1f,
    ];

    /// Creates a new CREATE3 miner with the specified parameters.
    ///
    /// # Arguments
    /// * `factory` - The address of the CREATE3 factory contract
    /// * `deployer` - The address that will call the factory contract
    pub fn new(deployer: Address, factory: Address) -> Self {
        Self { deployer, factory }
    }
}

impl Miner for Create3Miner {
    fn compute_address(&self, salt: &[u8; 32]) -> Address {
        unsafe {
            // Fills the remaining bytes with the salt
            SALT_BUFFER[20..].copy_from_slice(salt);

            // First deploy the proxy using CREATE2
            let proxy = self
                .factory
                .create2(keccak256(SALT_BUFFER), Self::PROXY_INIT_CODE_HASH);

            // Then compute the address the proxy would deploy using CREATE
            proxy.create(0x1)
        }
    }

    fn generate_salt_base(&self) -> [u8; 32] {
        [0u8; 32]
    }

    fn before_mine(&self) {
        // Fill the first 20 bytes with the deployer address
        unsafe {
            SALT_BUFFER[0..20].copy_from_slice(self.deployer.as_slice());
        }
    }
}

#[test]
fn test_create3_compute_address() {
    let deployer = address!("0x9fC3dc011b461664c835F2527fffb1169b3C213e");
    let factory = crate::CREATE3_DEFAULT_FACTORY;
    let miner = Create3Miner::new(deployer, factory);
    let salt = [2u8; 32];
    miner.before_mine();
    let computed = miner.compute_address(&salt);
    assert_eq!(
        computed,
        address!("0x1298be70f771753b5490b4708513d9f0F513dd36")
    );
}

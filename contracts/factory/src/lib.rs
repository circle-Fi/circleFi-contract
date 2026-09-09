#![no_std]
//! Deploys circles and keeps a public index of them.
//!
//! Without this, a circle can only be created by someone with a build
//! toolchain and a funded key, which puts the whole thing out of reach of the
//! people it is for. The factory turns creating a circle into one signed
//! transaction from a wallet, and gives the app a list to browse.
//!
//! The circle code itself is fixed at deploy: the factory stores one Wasm hash
//! and every circle it creates runs exactly that code. Nobody - including the
//! factory's admin - can point it at different code afterwards, so a circle
//! listed here is always the contract you can read in this repository.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
    Vec,
};

const DAY_IN_LEDGERS: u32 = 17_280;
const TTL_THRESHOLD: u32 = DAY_IN_LEDGERS * 30;
const TTL_EXTEND_TO: u32 = DAY_IN_LEDGERS * 120;

/// Bounded so `list` can never be asked for an unbounded amount of work.
const MAX_PAGE: u32 = 50;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Error {
    /// Contribution or round length was zero.
    InvalidTerms = 1,
    /// Circle size outside the range the circle contract accepts.
    InvalidCapacity = 2,
    /// Asked for more than MAX_PAGE circles at once.
    PageTooLarge = 3,
}

#[contracttype]
pub enum DataKey {
    /// Instance: the circle Wasm hash, fixed at deploy.
    Wasm,
    /// Instance: how many circles exist.
    Count,
    /// Persistent, one entry per circle - parallel-safe to read.
    Circle(u32),
}

/// A circle in the index, with the terms it was created on, so the app can
/// render a list without a round trip to every circle.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    pub address: Address,
    pub creator: Address,
    pub token: Address,
    pub contribution: i128,
    pub round_seconds: u64,
    pub capacity: u32,
    pub created_at: u64,
}

#[contract]
pub struct Factory;

#[contractimpl]
impl Factory {
    /// `circle_wasm` is the hash of an already-uploaded circle contract.
    pub fn __constructor(env: Env, circle_wasm: BytesN<32>) {
        env.storage().instance().set(&DataKey::Wasm, &circle_wasm);
        env.storage().instance().set(&DataKey::Count, &0u32);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
    }

    /// Create a circle and return its address.
    ///
    /// The caller signs, pays the fee, and becomes the new circle's admin. They
    /// get no special power over it - the circle has no privileged operations -
    /// but recording who opened it is worth having in the index.
    pub fn create(
        env: Env,
        creator: Address,
        token: Address,
        contribution: i128,
        round_seconds: u64,
        capacity: u32,
    ) -> Result<Address, Error> {
        creator.require_auth();

        if contribution <= 0 || round_seconds == 0 {
            return Err(Error::InvalidTerms);
        }
        if !(3..=24).contains(&capacity) {
            return Err(Error::InvalidCapacity);
        }

        let wasm: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::Wasm)
            .expect("factory not initialized");
        let index: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Count)
            .unwrap_or(0u32);

        // The index is the salt, so every circle gets a distinct address and
        // the same inputs never collide.
        let salt = BytesN::from_array(&env, &{
            let mut b = [0u8; 32];
            b[..4].copy_from_slice(&index.to_be_bytes());
            b
        });

        let address = env.deployer().with_current_contract(salt).deploy_v2(
            wasm,
            (
                creator.clone(),
                token.clone(),
                contribution,
                round_seconds,
                capacity,
            ),
        );

        let listing = Listing {
            address: address.clone(),
            creator,
            token,
            contribution,
            round_seconds,
            capacity,
            created_at: env.ledger().timestamp(),
        };
        let key = DataKey::Circle(index);
        env.storage().persistent().set(&key, &listing);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);

        env.storage().instance().set(&DataKey::Count, &(index + 1));
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);

        Ok(address)
    }

    pub fn count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::Count)
            .unwrap_or(0u32)
    }

    pub fn get(env: Env, index: u32) -> Option<Listing> {
        env.storage().persistent().get(&DataKey::Circle(index))
    }

    /// A page of the index, newest first, so the app's front page is one call.
    pub fn list(env: Env, offset: u32, limit: u32) -> Result<Vec<Listing>, Error> {
        if limit > MAX_PAGE {
            return Err(Error::PageTooLarge);
        }
        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Count)
            .unwrap_or(0u32);
        let mut out = Vec::new(&env);
        if offset >= count {
            return Ok(out);
        }
        // Newest first: walk down from the most recent index.
        let mut taken = 0u32;
        let mut i = count - offset;
        while i > 0 && taken < limit {
            i -= 1;
            if let Some(l) = env
                .storage()
                .persistent()
                .get::<DataKey, Listing>(&DataKey::Circle(i))
            {
                out.push_back(l);
                taken += 1;
            }
        }
        Ok(out)
    }

    /// The circle code every circle from this factory runs.
    pub fn circle_wasm(env: Env) -> BytesN<32> {
        env.storage()
            .instance()
            .get(&DataKey::Wasm)
            .unwrap_or_else(|| panic_with_error!(&env, Error::InvalidTerms))
    }
}

mod test;

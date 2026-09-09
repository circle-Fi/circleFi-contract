#![cfg(test)]

// The factory deploys real Wasm, so these tests need the circle contract built
// first:
//
//     cargo build -p circlefi-circle --target wasm32v1-none --release
//
// CI does that before `cargo test`. `cargo test -p circlefi-circle` needs
// nothing and covers the circle itself.
mod circle_wasm {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32v1-none/release/circlefi_circle.wasm"
    );
}

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

const CONTRIBUTION: i128 = 100;
const ROUND: u64 = 3600;

struct Fx {
    env: Env,
    client: FactoryClient<'static>,
    token: Address,
    user: Address,
}

fn setup() -> Fx {
    let env = Env::default();
    env.mock_all_auths();
    let issuer = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(issuer).address();
    let hash = env.deployer().upload_contract_wasm(circle_wasm::WASM);
    let id = env.register(Factory, (hash,));
    let user = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&user, &(CONTRIBUTION * 50));
    Fx {
        client: FactoryClient::new(&env, &id),
        token,
        user,
        env,
    }
}

#[test]
fn starts_empty() {
    let f = setup();
    assert_eq!(f.client.count(), 0);
    assert_eq!(f.client.list(&0, &10).len(), 0);
    assert_eq!(f.client.get(&0), None);
}

#[test]
fn creates_a_circle_that_is_a_real_working_circle() {
    let f = setup();
    let addr = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &3);

    // The deployed address is a live circle carrying the terms it was made with.
    let circle = circle_wasm::Client::new(&f.env, &addr);
    let cfg = circle.get_config();
    assert_eq!(cfg.contribution, CONTRIBUTION);
    assert_eq!(cfg.capacity, 3);
    assert_eq!(cfg.round_seconds, ROUND);
    assert_eq!(cfg.token, f.token);
    assert_eq!(cfg.admin, f.user);

    // And it is usable: the creator can join their own circle straight away.
    assert_eq!(circle.join(&f.user), 1);
    assert_eq!(circle.get_state().members.len(), 1);
}

#[test]
fn indexes_what_it_creates() {
    let f = setup();
    let a = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &3);

    assert_eq!(f.client.count(), 1);
    let l = f.client.get(&0).unwrap();
    assert_eq!(l.address, a);
    assert_eq!(l.creator, f.user);
    assert_eq!(l.capacity, 3);
    assert_eq!(l.contribution, CONTRIBUTION);
}

#[test]
fn every_circle_gets_its_own_address() {
    let f = setup();
    let a = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &3);
    // Identical terms must still produce a distinct circle, or the second
    // create would collide with the first.
    let b = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &3);
    assert_ne!(a, b);
    assert_eq!(f.client.count(), 2);
}

#[test]
fn lists_newest_first_and_pages() {
    let f = setup();
    let a = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &3);
    let b = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &4);
    let c = f.client.create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &5);

    let page = f.client.list(&0, &2);
    assert_eq!(page.len(), 2);
    assert_eq!(page.get(0).unwrap().address, c, "newest first");
    assert_eq!(page.get(1).unwrap().address, b);

    let rest = f.client.list(&2, &2);
    assert_eq!(rest.len(), 1);
    assert_eq!(rest.get(0).unwrap().address, a);

    assert_eq!(f.client.list(&99, &2).len(), 0, "past the end is empty");
}

#[test]
fn rejects_terms_the_circle_would_reject() {
    let f = setup();
    assert_eq!(
        f.client.try_create(&f.user, &f.token, &0i128, &ROUND, &3),
        Err(Ok(Error::InvalidTerms))
    );
    assert_eq!(
        f.client.try_create(&f.user, &f.token, &CONTRIBUTION, &0u64, &3),
        Err(Ok(Error::InvalidTerms))
    );
    assert_eq!(
        f.client.try_create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &2),
        Err(Ok(Error::InvalidCapacity))
    );
    assert_eq!(
        f.client.try_create(&f.user, &f.token, &CONTRIBUTION, &ROUND, &25),
        Err(Ok(Error::InvalidCapacity))
    );
    assert_eq!(f.client.count(), 0, "nothing was indexed");
}

#[test]
fn refuses_an_unbounded_page() {
    let f = setup();
    assert_eq!(f.client.try_list(&0, &51), Err(Ok(Error::PageTooLarge)));
}

#[test]
fn creating_requires_the_creator_to_sign() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(issuer).address();
    let hash = env.deployer().upload_contract_wasm(circle_wasm::WASM);
    let id = env.register(Factory, (hash,));
    let client = FactoryClient::new(&env, &id);
    let user = Address::generate(&env);

    // No auth mocked.
    assert!(client
        .try_create(&user, &token, &CONTRIBUTION, &ROUND, &3)
        .is_err());
}

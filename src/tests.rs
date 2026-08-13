#![cfg(test)]

use crate::*;
use soroban_sdk::Env;

#[test]
fn test_create_circle() {
    let env = Env::default();
    let contract = CirclefiContract;
    let owner = soroban_sdk::Address::random(&env);

    let result = contract.create_circle(
        env.clone(),
        owner.clone(),
        "Test Circle".into(),
        1000,
        30,
        PayoutOrder::Fixed,
        20,
        24,
        10,
        50,
    );

    assert!(result.is_ok());
}

#[test]
fn test_create_circle_invalid_amount() {
    let env = Env::default();
    let contract = CirclefiContract;
    let owner = soroban_sdk::Address::random(&env);

    let result = contract.create_circle(
        env.clone(),
        owner.clone(),
        "Test Circle".into(),
        -100, // Invalid negative amount
        30,
        PayoutOrder::Fixed,
        20,
        24,
        10,
        50,
    );

    assert!(result.is_err());
}

#[test]
fn test_join_circle() {
    let env = Env::default();
    let contract = CirclefiContract;
    let owner = soroban_sdk::Address::random(&env);
    let member = soroban_sdk::Address::random(&env);

    let _ = contract.create_circle(
        env.clone(),
        owner.clone(),
        "Test Circle".into(),
        1000,
        30,
        PayoutOrder::Fixed,
        20,
        24,
        10,
        50,
    );

    let result = contract.join_circle(env.clone(), owner.clone(), member.clone());

    assert!(result.is_ok());
}

#[test]
fn test_contribute() {
    let env = Env::default();
    let contract = CirclefiContract;
    let owner = soroban_sdk::Address::random(&env);
    let member = soroban_sdk::Address::random(&env);

    let _ = contract.create_circle(
        env.clone(),
        owner.clone(),
        "Test Circle".into(),
        1000,
        30,
        PayoutOrder::Fixed,
        20,
        24,
        10,
        50,
    );

    let _ = contract.join_circle(env.clone(), owner.clone(), member.clone());

    let result = contract.contribute(env.clone(), owner.clone(), member.clone(), 1000);

    assert!(result.is_ok());
}

#[test]
fn test_contribute_wrong_amount() {
    let env = Env::default();
    let contract = CirclefiContract;
    let owner = soroban_sdk::Address::random(&env);
    let member = soroban_sdk::Address::random(&env);

    let _ = contract.create_circle(
        env.clone(),
        owner.clone(),
        "Test Circle".into(),
        1000,
        30,
        PayoutOrder::Fixed,
        20,
        24,
        10,
        50,
    );

    let _ = contract.join_circle(env.clone(), owner.clone(), member.clone());

    let result = contract.contribute(env.clone(), owner.clone(), member.clone(), 500); // Wrong amount

    assert!(result.is_err());
}

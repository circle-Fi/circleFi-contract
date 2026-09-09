#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

const CONTRIBUTION: i128 = 100;
const ROUND_SECONDS: u64 = 30 * 24 * 60 * 60; // a month
const CAPACITY: u32 = 3;

struct Fixture {
    env: Env,
    client: CircleClient<'static>,
    token: token::Client<'static>,
    contract_id: Address,
    members: [Address; 3],
}

/// A funded three-member circle, not yet joined.
fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    let issuer = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(issuer).address();
    let minter = token::StellarAssetClient::new(&env, &token_id);

    let members = [
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];
    for m in members.iter() {
        minter.mint(m, &(CONTRIBUTION * 20));
    }

    let admin = Address::generate(&env);
    let contract_id = env.register(
        Circle,
        (
            admin,
            token_id.clone(),
            CONTRIBUTION,
            ROUND_SECONDS,
            CAPACITY,
        ),
    );

    Fixture {
        client: CircleClient::new(&env, &contract_id),
        token: token::Client::new(&env, &token_id),
        contract_id,
        members,
        env,
    }
}

impl Fixture {
    fn fill(&self) {
        for m in self.members.iter() {
            self.client.join(m);
        }
    }
    fn advance_past_round(&self) {
        let ends = self.client.get_state().round_ends_at;
        self.env.ledger().set_timestamp(ends + 1);
    }
    fn run_round(&self) {
        for m in self.members.iter() {
            self.client.contribute(m);
        }
        self.client.settle();
    }
}

// ---- creation ----

#[test]
fn creation_sets_terms_and_opens_for_members() {
    let f = setup();
    let cfg = f.client.get_config();
    assert_eq!(cfg.contribution, CONTRIBUTION);
    assert_eq!(cfg.capacity, CAPACITY);
    assert_eq!(cfg.round_seconds, ROUND_SECONDS);

    let st = f.client.get_state();
    assert_eq!(st.status, Status::Forming);
    assert_eq!(st.round, 0);
    assert_eq!(st.members.len(), 0);
}

#[test]
#[should_panic]
fn zero_contribution_is_rejected() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(issuer).address();
    let admin = Address::generate(&env);
    env.register(Circle, (admin, token_id, 0i128, ROUND_SECONDS, CAPACITY));
}

#[test]
#[should_panic]
fn two_member_circle_is_rejected() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(issuer).address();
    let admin = Address::generate(&env);
    env.register(Circle, (admin, token_id, CONTRIBUTION, ROUND_SECONDS, 2u32));
}

#[test]
#[should_panic]
fn oversized_circle_is_rejected() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(issuer).address();
    let admin = Address::generate(&env);
    env.register(
        Circle,
        (admin, token_id, CONTRIBUTION, ROUND_SECONDS, 25u32),
    );
}

// ---- joining ----

#[test]
fn joining_locks_a_deposit_and_returns_position() {
    let f = setup();
    let before = f.token.balance(&f.members[0]);

    assert_eq!(f.client.join(&f.members[0]), 1);
    assert_eq!(f.client.join(&f.members[1]), 2);

    assert_eq!(f.token.balance(&f.members[0]), before - CONTRIBUTION);
    assert_eq!(f.token.balance(&f.contract_id), CONTRIBUTION * 2);

    let rec = f.client.get_member(&f.members[0]).unwrap();
    assert_eq!(rec.deposit, CONTRIBUTION);
    assert_eq!(rec.defaults, 0);
    assert!(!rec.received);
}

#[test]
fn circle_activates_when_it_fills() {
    let f = setup();
    f.client.join(&f.members[0]);
    f.client.join(&f.members[1]);
    assert_eq!(f.client.get_state().status, Status::Forming);

    f.client.join(&f.members[2]);
    let st = f.client.get_state();
    assert_eq!(st.status, Status::Active);
    assert_eq!(st.round, 1);
    assert_eq!(st.round_ends_at, ROUND_SECONDS);
}

#[test]
fn cannot_join_twice() {
    let f = setup();
    f.client.join(&f.members[0]);
    assert_eq!(
        f.client.try_join(&f.members[0]),
        Err(Ok(Error::AlreadyMember))
    );
}

#[test]
fn cannot_join_a_running_circle() {
    let f = setup();
    f.fill();
    let outsider = Address::generate(&f.env);
    assert_eq!(f.client.try_join(&outsider), Err(Ok(Error::NotForming)));
}

#[test]
fn payout_order_is_join_order_and_is_visible_up_front() {
    let f = setup();
    f.fill();
    for (i, m) in f.members.iter().enumerate() {
        assert_eq!(f.client.recipient_of(&(i as u32 + 1)).unwrap(), *m);
    }
    assert_eq!(f.client.recipient_of(&0), None);
    assert_eq!(f.client.recipient_of(&99), None);
}

// ---- contributing ----

#[test]
fn cannot_contribute_before_the_circle_starts() {
    let f = setup();
    f.client.join(&f.members[0]);
    assert_eq!(
        f.client.try_contribute(&f.members[0]),
        Err(Ok(Error::NotActive))
    );
}

#[test]
fn non_members_cannot_contribute() {
    let f = setup();
    f.fill();
    let outsider = Address::generate(&f.env);
    assert_eq!(
        f.client.try_contribute(&outsider),
        Err(Ok(Error::NotMember))
    );
}

#[test]
fn cannot_contribute_twice_in_one_round() {
    let f = setup();
    f.fill();
    f.client.contribute(&f.members[0]);
    assert_eq!(
        f.client.try_contribute(&f.members[0]),
        Err(Ok(Error::AlreadyContributed))
    );
}

#[test]
fn contributing_moves_funds_and_is_recorded_per_round() {
    let f = setup();
    f.fill();
    let before = f.token.balance(&f.members[0]);

    f.client.contribute(&f.members[0]);

    assert_eq!(f.token.balance(&f.members[0]), before - CONTRIBUTION);
    assert!(f.client.has_contributed(&1, &f.members[0]));
    assert!(!f.client.has_contributed(&1, &f.members[1]));
    assert!(!f.client.has_contributed(&2, &f.members[0]));
}

// ---- settling ----

#[test]
fn cannot_settle_while_the_round_is_open_and_unpaid() {
    let f = setup();
    f.fill();
    f.client.contribute(&f.members[0]);
    assert_eq!(f.client.try_settle(), Err(Ok(Error::RoundStillOpen)));
}

#[test]
fn settles_early_once_everyone_has_paid() {
    let f = setup();
    f.fill();
    for m in f.members.iter() {
        f.client.contribute(m);
    }
    let before = f.token.balance(&f.members[0]);

    assert_eq!(f.client.settle(), f.members[0]);

    assert_eq!(
        f.token.balance(&f.members[0]),
        before + CONTRIBUTION * CAPACITY as i128
    );
    assert!(f.client.get_member(&f.members[0]).unwrap().received);
    assert_eq!(f.client.get_state().round, 2);
}

#[test]
fn a_missed_round_is_covered_by_the_defaulters_own_deposit() {
    let f = setup();
    f.fill();
    f.client.contribute(&f.members[0]);
    f.client.contribute(&f.members[2]);
    f.advance_past_round();

    let before = f.token.balance(&f.members[0]);
    f.client.settle();

    assert_eq!(
        f.token.balance(&f.members[0]),
        before + CONTRIBUTION * CAPACITY as i128,
        "recipient paid in full despite the default"
    );

    let defaulter = f.client.get_member(&f.members[1]).unwrap();
    assert_eq!(defaulter.deposit, 0, "deposit absorbed the miss");
    assert_eq!(defaulter.defaults, 1);
    assert!(
        !defaulter.delinquent,
        "deposit covered it, so not delinquent"
    );
}

#[test]
fn an_exhausted_deposit_leaves_the_pot_short_and_flags_the_member() {
    let f = setup();
    f.fill();

    f.client.contribute(&f.members[0]);
    f.client.contribute(&f.members[2]);
    f.advance_past_round();
    f.client.settle();

    f.client.contribute(&f.members[0]);
    f.client.contribute(&f.members[2]);
    f.advance_past_round();

    let before = f.token.balance(&f.members[1]);
    f.client.settle();

    assert_eq!(
        f.token.balance(&f.members[1]),
        before + CONTRIBUTION * 2,
        "pot is short by exactly the uncovered contribution"
    );
    let rec = f.client.get_member(&f.members[1]).unwrap();
    assert_eq!(rec.defaults, 2);
    assert!(rec.delinquent);
}

#[test]
fn topping_up_restores_the_deposit_and_clears_delinquency() {
    let f = setup();
    f.fill();
    f.client.contribute(&f.members[0]);
    f.client.contribute(&f.members[2]);
    f.advance_past_round();
    f.client.settle();

    assert_eq!(f.client.get_member(&f.members[1]).unwrap().deposit, 0);

    let before = f.token.balance(&f.members[1]);
    assert_eq!(f.client.top_up(&f.members[1]), CONTRIBUTION);

    assert_eq!(f.token.balance(&f.members[1]), before - CONTRIBUTION);
    let rec = f.client.get_member(&f.members[1]).unwrap();
    assert_eq!(rec.deposit, CONTRIBUTION);
    assert!(!rec.delinquent);
    assert_eq!(rec.defaults, 1, "paying up does not erase the history");
}

#[test]
fn cannot_top_up_an_intact_deposit() {
    let f = setup();
    f.fill();
    assert_eq!(
        f.client.try_top_up(&f.members[0]),
        Err(Ok(Error::DepositIntact))
    );
}

// ---- full cycle ----

#[test]
fn every_member_receives_exactly_once_and_the_books_balance() {
    let f = setup();
    let opening: [i128; 3] = [
        f.token.balance(&f.members[0]),
        f.token.balance(&f.members[1]),
        f.token.balance(&f.members[2]),
    ];
    f.fill();

    for round in 1..=CAPACITY {
        for m in f.members.iter() {
            f.client.contribute(m);
        }
        assert_eq!(f.client.settle(), f.members[(round - 1) as usize]);
    }

    assert_eq!(f.client.get_state().status, Status::Complete);

    for m in f.members.iter() {
        assert!(f.client.get_member(m).unwrap().received);
        f.client.withdraw_deposit(m);
    }

    for (i, m) in f.members.iter().enumerate() {
        assert_eq!(f.token.balance(m), opening[i], "member {i} is square");
    }
    assert_eq!(f.token.balance(&f.contract_id), 0, "contract fully drained");
}

#[test]
fn a_defaulter_ends_down_exactly_what_they_failed_to_pay() {
    let f = setup();
    let opening = f.token.balance(&f.members[1]);
    f.fill();

    f.client.contribute(&f.members[0]);
    f.client.contribute(&f.members[2]);
    f.advance_past_round();
    f.client.settle();

    for _ in 2..=CAPACITY {
        f.run_round();
    }

    assert_eq!(f.client.get_state().status, Status::Complete);
    assert_eq!(
        f.client.try_withdraw_deposit(&f.members[1]),
        Err(Ok(Error::NothingToWithdraw))
    );
    assert_eq!(f.token.balance(&f.members[1]), opening);
}

// ---- withdrawal ----

#[test]
fn cannot_withdraw_before_completion() {
    let f = setup();
    f.fill();
    assert_eq!(
        f.client.try_withdraw_deposit(&f.members[0]),
        Err(Ok(Error::NotComplete))
    );
}

#[test]
fn cannot_withdraw_twice() {
    let f = setup();
    f.fill();
    for _ in 1..=CAPACITY {
        f.run_round();
    }
    f.client.withdraw_deposit(&f.members[0]);
    assert_eq!(
        f.client.try_withdraw_deposit(&f.members[0]),
        Err(Ok(Error::NothingToWithdraw))
    );
}

#[test]
fn cannot_settle_a_completed_circle() {
    let f = setup();
    f.fill();
    for _ in 1..=CAPACITY {
        f.run_round();
    }
    assert_eq!(f.client.try_settle(), Err(Ok(Error::NotActive)));
}

// ---- authorisation ----

#[test]
fn joining_requires_the_members_own_signature() {
    let env = Env::default();
    let issuer = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(issuer).address();
    let admin = Address::generate(&env);
    let member = Address::generate(&env);
    let contract_id = env.register(
        Circle,
        (admin, token_id, CONTRIBUTION, ROUND_SECONDS, CAPACITY),
    );
    let client = CircleClient::new(&env, &contract_id);

    // No auth mocked: the call must not go through.
    assert!(client.try_join(&member).is_err());
}

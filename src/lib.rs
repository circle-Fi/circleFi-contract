#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, i128, Symbol};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[contracttype]
pub enum DataKey {
    Circle(Address),
    Member(Address, Address), // (circle, member)
    Round(Address, u32),       // (circle, round_id)
}

#[derive(Clone)]
#[contracttype]
pub struct Circle {
    pub owner: Address,
    pub name: String,
    pub contribution_amount: i128,
    pub cycle_duration_days: u32,
    pub payout_order: PayoutOrder,
    pub collateral_percentage: u32,
    pub dispute_window_hours: u32,
    pub current_round: u32,
    pub is_active: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum PayoutOrder {
    Fixed,
    Bid,
}

#[derive(Clone)]
#[contracttype]
pub struct Member {
    pub address: Address,
    pub contributions_made: u32,
    pub payouts_received: u32,
    pub collateral_locked: i128,
    pub default_count: u32,
    pub trust_score: u32, // 0-100
    pub joined_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct Round {
    pub circle: Address,
    pub round_id: u32,
    pub start_time: u64,
    pub end_time: u64,
    pub total_pot: i128,
    pub payout_recipient: Address,
    pub highest_bid: i128,
    pub contributions_received: u32,
    pub disputed: bool,
}

#[contract]
pub struct CirclefiContract;

#[contractimpl]
impl CirclefiContract {
    /// Create a new savings circle
    pub fn create_circle(
        env: Env,
        owner: Address,
        name: String,
        contribution_amount: i128,
        cycle_duration_days: u32,
        payout_order: PayoutOrder,
        collateral_percentage: u32,
        dispute_window_hours: u32,
    ) -> Address {
        owner.require_auth();

        let circle = Circle {
            owner: owner.clone(),
            name,
            contribution_amount,
            cycle_duration_days,
            payout_order,
            collateral_percentage,
            dispute_window_hours,
            current_round: 0,
            is_active: true,
        };

        env.storage().instance().set(&DataKey::Circle(owner.clone()), &circle);
        owner
    }

    /// Join a circle as a member
    pub fn join_circle(env: Env, circle_owner: Address, member: Address) {
        member.require_auth();

        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .expect("Circle not found");

        let member_obj = Member {
            address: member.clone(),
            contributions_made: 0,
            payouts_received: 0,
            collateral_locked: 0,
            default_count: 0,
            trust_score: 100,
            joined_at: env.ledger().timestamp(),
        };

        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner, member), &member_obj);
    }

    /// Record a contribution from a member
    pub fn contribute(env: Env, circle_owner: Address, member: Address, amount: i128) {
        member.require_auth();

        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .expect("Circle not found");

        soroban_sdk::assert!(amount == circle.contribution_amount, "Invalid contribution amount");

        let mut member_obj: Member = env
            .storage()
            .instance()
            .get(&DataKey::Member(circle_owner.clone(), member.clone()))
            .expect("Member not found in circle");

        member_obj.contributions_made += 1;

        // Update round pot
        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), circle.current_round))
            .unwrap_or_else(|| Round {
                circle: circle_owner.clone(),
                round_id: circle.current_round,
                start_time: env.ledger().timestamp(),
                end_time: env.ledger().timestamp() + (circle.cycle_duration_days as u64 * 86400),
                total_pot: 0,
                payout_recipient: member.clone(),
                highest_bid: 0,
                contributions_received: 0,
                disputed: false,
            });

        round.total_pot += amount;
        round.contributions_received += 1;

        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner.clone(), member), &member_obj);
        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, circle.current_round), &round);
    }

    /// Retrieve circle information
    pub fn get_circle(env: Env, circle_owner: Address) -> Circle {
        env.storage()
            .instance()
            .get(&DataKey::Circle(circle_owner))
            .expect("Circle not found")
    }

    /// Retrieve member information
    pub fn get_member(env: Env, circle_owner: Address, member: Address) -> Member {
        env.storage()
            .instance()
            .get(&DataKey::Member(circle_owner, member))
            .expect("Member not found")
    }

    /// Retrieve round information
    pub fn get_round(env: Env, circle_owner: Address, round_id: u32) -> Round {
        env.storage()
            .instance()
            .get(&DataKey::Round(circle_owner, round_id))
            .expect("Round not found")
    }

    /// Place a bid for payout (bid-based payout order)
    pub fn place_bid(env: Env, circle_owner: Address, member: Address, bid_amount: i128) {
        member.require_auth();

        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .expect("Circle not found");

        soroban_sdk::assert!(
            circle.payout_order == PayoutOrder::Bid,
            "Bidding not enabled for this circle"
        );

        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), circle.current_round))
            .expect("Round not found");

        if bid_amount > round.highest_bid {
            round.highest_bid = bid_amount;
            round.payout_recipient = member;
        }

        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, circle.current_round), &round);
    }

    /// Trigger default/slashing for non-payment
    pub fn trigger_default(env: Env, circle_owner: Address, member: Address) {
        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .expect("Circle not found");

        circle.owner.require_auth();

        let mut member_obj: Member = env
            .storage()
            .instance()
            .get(&DataKey::Member(circle_owner.clone(), member.clone()))
            .expect("Member not found");

        member_obj.default_count += 1;
        member_obj.collateral_locked = (circle.contribution_amount * circle.collateral_percentage as i128) / 100;

        // Reduce trust score
        if member_obj.trust_score > 10 {
            member_obj.trust_score -= 10;
        }

        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner, member), &member_obj);
    }

    /// Dispute a payout decision
    pub fn dispute_payout(env: Env, circle_owner: Address, disputer: Address, round_id: u32) {
        disputer.require_auth();

        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), round_id))
            .expect("Round not found");

        let current_time = env.ledger().timestamp();
        let dispute_window_seconds = round.circle.to_xdr().len() as u64; // Placeholder

        soroban_sdk::assert!(!round.disputed, "Round already disputed");

        round.disputed = true;

        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, round_id), &round);
    }

    /// End a round and move to next
    pub fn end_round(env: Env, circle_owner: Address) {
        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .expect("Circle not found");

        circle.owner.require_auth();

        let mut updated_circle = circle.clone();
        updated_circle.current_round += 1;

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_owner), &updated_circle);
    }
}

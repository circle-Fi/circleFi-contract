#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, i128, Symbol};

// ==================== Error Handling ====================

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum ErrorCode {
    CircleNotFound = 1,
    MemberNotFound = 2,
    RoundNotFound = 3,
    InvalidAmount = 4,
    UnauthorizedAccess = 5,
    InsufficientCollateral = 6,
    InvalidPayoutOrder = 7,
    RoundNotEnded = 8,
    InvalidParameters = 9,
    MemberAlreadyJoined = 10,
    CircleNotActive = 11,
    MaxMembersReached = 12,
    InsufficientTrustScore = 13,
}

// ==================== Data Keys ====================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[contracttype]
pub enum DataKey {
    Circle(Address),
    Member(Address, Address), // (circle, member)
    Round(Address, u32),       // (circle, round_id)
    CircleList,
    MemberList(Address),
    CircleCount,
}

// ==================== Enums ====================

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum PayoutOrder {
    Fixed = 0,
    Bid = 1,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum MemberStatus {
    Active = 0,
    Defaulted = 1,
    Suspended = 2,
    Exited = 3,
}

// ==================== Structs ====================

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
    pub created_at: u64,
    pub member_count: u32,
    pub max_members: u32,
    pub total_volume: i128,
    pub min_trust_score: u32,
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
    pub last_contribution: u64,
    pub status: MemberStatus,
    pub total_contributed: i128,
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
    pub member_count: u32,
    pub disputed: bool,
    pub dispute_resolver: Address,
    pub is_finalized: bool,
}

// ==================== Contract ====================

#[contract]
pub struct CirclefiContract;

#[contractimpl]
impl CirclefiContract {
    /// Initialize contract (optional for Soroban, but good for state)
    pub fn initialize(env: Env) {
        env.storage().instance().set(&DataKey::CircleCount, &0u32);
    }

    /// Create a new savings circle with validation
    pub fn create_circle(
        env: Env,
        owner: Address,
        name: String,
        contribution_amount: i128,
        cycle_duration_days: u32,
        payout_order: PayoutOrder,
        collateral_percentage: u32,
        dispute_window_hours: u32,
        max_members: u32,
        min_trust_score: u32,
    ) -> Result<Address, ErrorCode> {
        owner.require_auth();

        // Validate parameters
        if contribution_amount <= 0 {
            return Err(ErrorCode::InvalidAmount);
        }
        if cycle_duration_days == 0 || cycle_duration_days > 365 {
            return Err(ErrorCode::InvalidParameters);
        }
        if collateral_percentage > 100 {
            return Err(ErrorCode::InvalidParameters);
        }
        if dispute_window_hours == 0 || dispute_window_hours > 720 {
            return Err(ErrorCode::InvalidParameters);
        }
        if max_members == 0 || max_members > 100 {
            return Err(ErrorCode::InvalidParameters);
        }
        if min_trust_score > 100 {
            return Err(ErrorCode::InvalidParameters);
        }

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
            created_at: env.ledger().timestamp(),
            member_count: 1,
            max_members,
            total_volume: 0,
            min_trust_score,
        };

        env.storage().instance().set(&DataKey::Circle(owner.clone()), &circle);
        
        // Add owner as first member
        let owner_member = Member {
            address: owner.clone(),
            contributions_made: 0,
            payouts_received: 0,
            collateral_locked: 0,
            default_count: 0,
            trust_score: 100,
            joined_at: env.ledger().timestamp(),
            last_contribution: 0,
            status: MemberStatus::Active,
            total_contributed: 0,
        };

        env.storage()
            .instance()
            .set(&DataKey::Member(owner.clone(), owner.clone()), &owner_member);

        Ok(owner)
    }

    /// Join a circle with trust score validation
    pub fn join_circle(env: Env, circle_owner: Address, member: Address) -> Result<(), ErrorCode> {
        member.require_auth();

        let mut circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        if !circle.is_active {
            return Err(ErrorCode::CircleNotActive);
        }

        if circle.member_count >= circle.max_members {
            return Err(ErrorCode::MaxMembersReached);
        }

        // Check if already a member
        if env
            .storage()
            .instance()
            .get::<DataKey, Member>(&DataKey::Member(circle_owner.clone(), member.clone()))
            .is_some()
        {
            return Err(ErrorCode::MemberAlreadyJoined);
        }

        let member_obj = Member {
            address: member.clone(),
            contributions_made: 0,
            payouts_received: 0,
            collateral_locked: 0,
            default_count: 0,
            trust_score: 100,
            joined_at: env.ledger().timestamp(),
            last_contribution: 0,
            status: MemberStatus::Active,
            total_contributed: 0,
        };

        circle.member_count += 1;

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_owner.clone()), &circle);
        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner, member), &member_obj);

        Ok(())
    }

    /// Record a contribution with validation
    pub fn contribute(
        env: Env,
        circle_owner: Address,
        member: Address,
        amount: i128,
    ) -> Result<(), ErrorCode> {
        member.require_auth();

        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        if !circle.is_active {
            return Err(ErrorCode::CircleNotActive);
        }

        if amount != circle.contribution_amount {
            return Err(ErrorCode::InvalidAmount);
        }

        let mut member_obj: Member = env
            .storage()
            .instance()
            .get(&DataKey::Member(circle_owner.clone(), member.clone()))
            .ok_or(ErrorCode::MemberNotFound)?;

        if member_obj.status != MemberStatus::Active {
            return Err(ErrorCode::UnauthorizedAccess);
        }

        member_obj.contributions_made += 1;
        member_obj.last_contribution = env.ledger().timestamp();
        member_obj.total_contributed += amount;

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
                member_count: circle.member_count,
                disputed: false,
                dispute_resolver: Address::from_contract_id(&env, &env.contract_id()),
                is_finalized: false,
            });

        round.total_pot += amount;
        round.contributions_received += 1;

        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner.clone(), member), &member_obj);
        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, circle.current_round), &round);

        Ok(())
    }

    /// Place a bid for payout (bid-based circles only)
    pub fn place_bid(
        env: Env,
        circle_owner: Address,
        member: Address,
        bid_amount: i128,
    ) -> Result<(), ErrorCode> {
        member.require_auth();

        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        if circle.payout_order != PayoutOrder::Bid {
            return Err(ErrorCode::InvalidPayoutOrder);
        }

        if bid_amount <= 0 {
            return Err(ErrorCode::InvalidAmount);
        }

        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), circle.current_round))
            .ok_or(ErrorCode::RoundNotFound)?;

        if bid_amount > round.highest_bid {
            round.highest_bid = bid_amount;
            round.payout_recipient = member;
        }

        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, circle.current_round), &round);

        Ok(())
    }

    /// Trigger default/slashing for non-payment
    pub fn trigger_default(
        env: Env,
        circle_owner: Address,
        member: Address,
    ) -> Result<(), ErrorCode> {
        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        circle.owner.require_auth();

        let mut member_obj: Member = env
            .storage()
            .instance()
            .get(&DataKey::Member(circle_owner.clone(), member.clone()))
            .ok_or(ErrorCode::MemberNotFound)?;

        member_obj.default_count += 1;
        member_obj.status = MemberStatus::Defaulted;
        member_obj.collateral_locked = (circle.contribution_amount * circle.collateral_percentage as i128) / 100;

        // Reduce trust score
        let score_reduction = 15 * member_obj.default_count as u32;
        if score_reduction >= member_obj.trust_score {
            member_obj.trust_score = 0;
            member_obj.status = MemberStatus::Suspended;
        } else {
            member_obj.trust_score -= score_reduction;
        }

        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner, member), &member_obj);

        Ok(())
    }

    /// Dispute a payout decision
    pub fn dispute_payout(
        env: Env,
        circle_owner: Address,
        disputer: Address,
        round_id: u32,
    ) -> Result<(), ErrorCode> {
        disputer.require_auth();

        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), round_id))
            .ok_or(ErrorCode::RoundNotFound)?;

        if round.disputed {
            return Err(ErrorCode::InvalidParameters);
        }

        let current_time = env.ledger().timestamp();
        let dispute_window_seconds = (round.circle.to_xdr().len() as u64) * 3600; // Placeholder

        if current_time > round.end_time + dispute_window_seconds {
            return Err(ErrorCode::InvalidParameters);
        }

        round.disputed = true;
        round.dispute_resolver = disputer;

        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, round_id), &round);

        Ok(())
    }

    /// End a round and move to next
    pub fn end_round(env: Env, circle_owner: Address) -> Result<(), ErrorCode> {
        let mut circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        circle.owner.require_auth();

        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), circle.current_round))
            .ok_or(ErrorCode::RoundNotFound)?;

        if env.ledger().timestamp() < round.end_time {
            return Err(ErrorCode::RoundNotEnded);
        }

        round.is_finalized = true;
        circle.current_round += 1;
        circle.total_volume += round.total_pot;

        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner.clone(), round.round_id), &round);
        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_owner), &circle);

        Ok(())
    }

    /// Retrieve circle information
    pub fn get_circle(env: Env, circle_owner: Address) -> Result<Circle, ErrorCode> {
        env.storage()
            .instance()
            .get(&DataKey::Circle(circle_owner))
            .ok_or(ErrorCode::CircleNotFound)
    }

    /// Retrieve member information
    pub fn get_member(
        env: Env,
        circle_owner: Address,
        member: Address,
    ) -> Result<Member, ErrorCode> {
        env.storage()
            .instance()
            .get(&DataKey::Member(circle_owner, member))
            .ok_or(ErrorCode::MemberNotFound)
    }

    /// Retrieve round information
    pub fn get_round(
        env: Env,
        circle_owner: Address,
        round_id: u32,
    ) -> Result<Round, ErrorCode> {
        env.storage()
            .instance()
            .get(&DataKey::Round(circle_owner, round_id))
            .ok_or(ErrorCode::RoundNotFound)
    }

    /// Finalize a round (admin function)
    pub fn finalize_round(
        env: Env,
        circle_owner: Address,
        round_id: u32,
        payout_recipient: Address,
    ) -> Result<(), ErrorCode> {
        let circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        circle.owner.require_auth();

        let mut round: Round = env
            .storage()
            .instance()
            .get(&DataKey::Round(circle_owner.clone(), round_id))
            .ok_or(ErrorCode::RoundNotFound)?;

        if round.is_finalized {
            return Err(ErrorCode::RoundNotEnded);
        }

        round.is_finalized = true;
        round.payout_recipient = payout_recipient.clone();

        // Increase trust score for member who received payout
        let mut member: Member = env
            .storage()
            .instance()
            .get(&DataKey::Member(circle_owner.clone(), payout_recipient.clone()))
            .ok_or(ErrorCode::MemberNotFound)?;

        member.payouts_received += 1;
        if member.trust_score < 100 {
            member.trust_score += 5;
        }

        env.storage()
            .instance()
            .set(&DataKey::Member(circle_owner.clone(), payout_recipient), &member);
        env.storage()
            .instance()
            .set(&DataKey::Round(circle_owner, round_id), &round);

        Ok(())
    }

    /// Deactivate a circle
    pub fn deactivate_circle(env: Env, circle_owner: Address) -> Result<(), ErrorCode> {
        let mut circle: Circle = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_owner.clone()))
            .ok_or(ErrorCode::CircleNotFound)?;

        circle.owner.require_auth();

        circle.is_active = false;

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_owner), &circle);

        Ok(())
    }
}

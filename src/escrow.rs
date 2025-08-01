use crate::timelocks::{TimelockDelays, Timelocks};
use near_sdk::{json_types::Base58CryptoHash, near, AccountId, CryptoHash, NearToken};

pub type EscrowId = CryptoHash;

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub enum Asset {
    Ft(AccountId),
}

impl Asset {
    pub fn ft_token_id(&self) -> AccountId {
        match self {
            Asset::Ft(id) => id.clone(),
        }
    }
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Escrow {
    pub hashlock: CryptoHash,
    pub maker: AccountId,
    pub taker: AccountId,
    pub asset: Asset,
    pub amount: NearToken,
    pub timelocks: Timelocks,
    pub safety_deposit: NearToken,
    pub claimed: bool,
    pub is_source: bool,
}

/// Defines the messages passed via `ft_transfer_call`.
#[near(serializers = [json])]
#[serde(tag = "type")]
pub enum FtMessage {
    /// A simple deposit to the user's internal balance.
    Deposit,
    /// Creates a destination-side escrow (e.g., for an ETH -> NEAR swap).
    CreateDestinationEscrow {
        hashlock: Base58CryptoHash,
        maker_id: AccountId,
        timelocks: TimelockDelays,
    },
}

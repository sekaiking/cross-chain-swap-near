use near_sdk::{near, AccountId, CryptoHash, NearToken, Timestamp};

use crate::timelocks::TimelockDelays;

use super::timelocks::Timelocks;

// Unique identifier for a swap. We are using SHA256 hash of the secret.
pub type EscrowId = CryptoHash;

// NEP-141 Fungible Token or Native NEAR
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub enum Asset {
    Native,
    Ft(AccountId),
}

// All the immutable parameters of a single swap instance.
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Escrow {
    // Core HTLC parameters
    pub hashlock: CryptoHash,
    pub maker: AccountId,          // The initiator of the swap (the user)
    pub taker: AccountId,          // The party filling the swap (the resolver)
    pub asset: Asset,
    pub amount: NearToken,

    // Timelock
    pub timelocks: Timelocks,

    // Incentives & State
    pub safety_deposit: NearToken,
    pub claimed: bool,          // Flag to prevent double-spends before deletion
    pub is_source: bool,       // Flag to distinguish swap direction
}

// Message for ft_on_transfer to initiate an FT swap
#[near(serializers = [json, borsh])]
pub struct FtOnTransferMsg {
    pub hashlock: near_sdk::json_types::Base58CryptoHash,
    pub taker: AccountId,
    pub timelocks: TimelockDelays,
    pub is_source: bool,
    // safety_deposit is sent as native NEAR attached to the ft_transfer_call
}

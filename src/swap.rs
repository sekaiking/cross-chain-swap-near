use near_sdk::{near, AccountId, CryptoHash, NearToken, Timestamp};

// Unique identifier for a swap. We are using SHA256 hash of the secret.
pub type SwapId = CryptoHash;

// NEP-141 Fungible Token or Native NEAR
#[near(serializers = [json, borsh])]
pub enum Asset {
    Native,
    Ft(AccountId),
}

// All the immutable parameters of a single swap instance.
#[near(serializers = [json, borsh])]
pub struct Swap {
    // Core HTLC parameters
    pub hashlock: CryptoHash,
    pub maker: AccountId,          // The initiator of the swap (the user)
    pub taker: AccountId,          // The party filling the swap (the resolver)
    pub asset: Asset,
    pub amount: NearToken,

    // Timelock
    pub timeout: Timestamp,

    // Incentives & State
    pub safety_deposit: NearToken,
    pub claimed: bool,          // Flag to prevent double-spends before deletion
}

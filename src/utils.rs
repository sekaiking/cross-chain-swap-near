use near_sdk::{env, CryptoHash, AccountId, NearToken, bs58};

// Helper for consistent logging
pub fn log_escrow_event(event: &str, hashlock: &CryptoHash, actor: &AccountId, amount: NearToken) {
    env::log_str(&format!(
        "ESCROW_{}: hashlock='{}', actor='{}', amount='{}'",
        event, bs58::encode(hashlock).into_string(), actor, amount.as_yoctonear()
    ));
}


use super::timelocks::TimelockDelays;
use near_sdk::{
    borsh::BorshSerialize, env, near, require, store::IterableSet, AccountId, PublicKey,
};

/// The core off-chain order signed by the maker.
/// This struct must be serialized with a canonical format (Borsh) for the signature to be valid.
/// It contains all parameters the maker agrees to for a single side of the swap.
#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct SignedOrder {
    pub nonce: u128,
    pub maker_id: AccountId,
    pub taker_id: AccountId,
    pub asset_id: AccountId, // For FTs: the token contract ID. For native NEAR: `near` or this contract's ID.
    pub amount: u128,
    pub hashlock: near_sdk::json_types::Base58CryptoHash,
    pub timelocks: TimelockDelays,
    pub is_source: bool,
}

impl SignedOrder {
    /// Serializes the params into a canonical byte array for signing/verification.
    pub fn to_message_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        self.serialize(&mut buffer).expect("Serialization failed");
        buffer
    }
}

/// Verifies that the predecessor (resolver) has a valid signature from the maker
/// to execute the escrow creation.
pub fn verify_maker_signature(
    params: &SignedOrder,
    signature_bytes: &[u8],
    public_key: &PublicKey,
    used_nonces: &mut IterableSet<u128>,
) {
    require!(
        env::predecessor_account_id() == params.taker_id,
        "Caller is not the designated resolver"
    );
    require!(!used_nonces.contains(&params.nonce), "Nonce already used");

    let message_bytes = params.to_message_bytes();
    let message_hash = env::sha256(&message_bytes);

    let signature: [u8; 64] = signature_bytes
        .try_into()
        .expect("Signature must be 64 bytes");

    let pk_bytes: Vec<u8> = public_key.clone().into();
    let public_key_arr: [u8; 32] = pk_bytes[1..].try_into().expect("Invalid public key format");

    require!(
        env::ed25519_verify(&signature, &message_hash, &public_key_arr),
        "Signature verification failed"
    );

    used_nonces.insert(params.nonce);
}

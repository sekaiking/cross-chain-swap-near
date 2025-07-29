use near_sdk::env::{log_str, panic_str};
use near_sdk::json_types::{Base58CryptoHash, U128};
use near_sdk::store::{IterableMap, IterableSet};
use near_sdk::{
    base64, bs58, env, ext_contract, log, near, require, serde_json, AccountId, NearToken, Promise,
    PromiseOrValue, PromiseResult, PublicKey,
};

mod escrow;
mod signatures;
mod timelocks;
mod utils;

use escrow::{Asset, Escrow, EscrowId};
use signatures::{verify_maker_signature, SignedOrder};
use timelocks::Timelocks;
use utils::log_escrow_event;

use crate::escrow::FtOnTransferMsg;

// External contract interfaces
#[ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_from(
        &mut self,
        owner_id: AccountId,
        new_owner_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
}

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn on_escrow_settled(&mut self, hashlock: EscrowId);
    fn on_ft_pulled_for_escrow(&mut self, params: SignedOrder, safety_deposit: NearToken);
}

// Define the contract structure
#[near(contract_state)]
pub struct Contract {
    pub owner_id: AccountId,
    // The main collection storing all active escrows, keyed by their EscrowId (secret_hash)
    pub escrows: IterableMap<EscrowId, Escrow>,
    pub used_nonces: IterableSet<u128>,
    pub registered_keys: IterableMap<AccountId, Vec<PublicKey>>,
}

// Define the default, which automatically initializes the contract
impl Default for Contract {
    fn default() -> Self {
        Self {
            owner_id: env::predecessor_account_id(),
            escrows: IterableMap::new(b"e"),
            used_nonces: IterableSet::new(b"n"),
            registered_keys: IterableMap::new(b"k"),
        }
    }
}

// Implement the contract structure
#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            escrows: IterableMap::new(b"e"),
            used_nonces: IterableSet::new(b"n"),
            registered_keys: IterableMap::new(b"k"),
        }
    }

    /// Allows a user to register a FullAccess public key with the contract.
    /// This proves they own the key and the account. This is a one-time setup.
    pub fn register_key(&mut self) {
        let account_id = env::signer_account_id();
        let public_key = env::signer_account_pk();

        let keys = self.registered_keys.get_mut(&account_id);

        if let Some(keys) = keys {
            if !keys.contains(&public_key) {
                keys.push(public_key.clone());
            }
        } else {
            self.registered_keys
                .insert(account_id.clone(), vec![public_key.clone()]);
        }

        log_str(&format!(
            "Registered key {} for account {}",
            bs58::encode(public_key.as_bytes()).into_string(),
            account_id,
        ));
    }

    /// Creates a SOURCE escrow (e.g., NEAR -> ETH).
    /// Called by a Resolver who presents a Maker's signed intent.
    /// The contract pulls tokens from the Maker's account using an allowance.
    #[payable]
    pub fn initiate_source_escrow(
        &mut self,
        params: SignedOrder,
        signature: String,
        public_key: PublicKey,
    ) -> Promise {
        let safety_deposit = env::attached_deposit();
        require!(
            safety_deposit.as_yoctonear() > 0,
            "A native NEAR safety deposit must be attached by the resolver"
        );

        // Verify the public key is registered for the maker
        let maker_keys = self
            .registered_keys
            .get(&params.maker_id)
            .expect("No keys registered for this maker");
        require!(
            maker_keys.contains(&public_key),
            "Public key not registered for this maker"
        );

        // Signature verification & nonce consumption happens inside this function
        let signature_bytes = base64::decode(&signature).expect("Invalid signature format");
        verify_maker_signature(
            &params,
            &signature_bytes,
            &public_key,
            &mut self.used_nonces,
        );

        // Make sure timelocks are valide dates
        params.timelocks.validate();

        let hashlock_bytes: EscrowId = params.hashlock.into();
        require!(
            !self.escrows.contains_key(&hashlock_bytes),
            "Escrow with this hashlock already exists"
        );
        require!(params.is_source, "This function is for source escrows only");

        // Trigger the token pull from the Maker's account.
        ext_fungible_token::ext(params.asset_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(env::prepaid_gas().saturating_div(4))
            .ft_transfer_from(
                params.maker_id.clone(),
                env::current_account_id(),
                U128(params.amount),
                Some("1inch Fusion+ Escrow".to_string()),
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(env::prepaid_gas().saturating_div(4))
                    .on_ft_pulled_for_escrow(params, safety_deposit),
            )
    }

    /// Creates a DESTINATION escrow (e.g., ETH -> NEAR).
    ///
    /// NEP-141 Receiver: Initiates an escrow with a Fungible Token.
    ///
    /// To escrow a Fungible Token, the user (or resolver script acting on their behalf)
    /// does not call the contract directly. Instead, they execute a single transaction by calling
    /// the ft_transfer_call function on the token contract itself.
    #[payable]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let resolver_id = sender_id;
        let token_contract_id = env::predecessor_account_id();
        let safety_deposit = env::attached_deposit();
        require!(
            safety_deposit.as_yoctonear() > 0,
            "A native NEAR safety deposit must be attached"
        );

        let params: FtOnTransferMsg =
            serde_json::from_str(&msg).expect("Invalid params format for destination escrow");

        let hashlock_bytes: EscrowId = params.hashlock.into();
        require!(
            !self.escrows.contains_key(&hashlock_bytes),
            "Escrow with this hashlock already exists"
        );

        params.timelocks.validate();

        let escrow = Escrow {
            hashlock: hashlock_bytes,
            maker: params.maker_id,
            taker: resolver_id.clone(),
            asset: Asset::Ft(token_contract_id),
            amount: NearToken::from_yoctonear(amount.0),
            safety_deposit,
            is_source: false,
            timelocks: Timelocks::new(env::block_timestamp(), params.timelocks),
            claimed: false,
        };

        self.escrows.insert(hashlock_bytes, escrow);
        log_escrow_event(
            "INITIATED_DESTINATION",
            &hashlock_bytes,
            &resolver_id,
            NearToken::from_yoctonear(amount.0),
        );

        PromiseOrValue::Value(U128(0))
    }

    pub fn withdraw(&mut self, secret: String) -> Promise {
        let secret_bytes = base64::decode(secret).expect("Invalid base64 secret");
        let hashlock_bytes: EscrowId = env::sha256_array(&secret_bytes);

        let mut escrow = self
            .escrows
            .get(&hashlock_bytes)
            .cloned()
            .expect("Escrow not found");

        require!(!escrow.claimed, "Escrow already claimed");

        let is_public_caller = env::predecessor_account_id() != escrow.taker;

        if escrow.is_source {
            escrow
                .timelocks
                .assert_src_withdrawal_window(is_public_caller)
        } else {
            escrow
                .timelocks
                .assert_dst_withdrawal_window(is_public_caller)
        }

        escrow.claimed = true;
        self.escrows.insert(hashlock_bytes, escrow.clone());

        let caller = env::predecessor_account_id();
        let recipient = if escrow.is_source {
            escrow.taker.clone() // Source: Taker claims
        } else {
            escrow.maker.clone() // Destination: Maker claims
        };

        let main_transfer = match escrow.asset.clone() {
            Asset::Native => Promise::new(recipient).transfer(escrow.amount),
            Asset::Ft(token_id) => ext_fungible_token::ext(token_id)
                .with_static_gas(env::prepaid_gas().saturating_div(4))
                .ft_transfer(
                    recipient,
                    U128(escrow.amount.as_yoctonear()),
                    Some("1inch Fusion+ Swap".to_string()),
                ),
        };

        let safety_deposit_transfer = Promise::new(caller.clone()).transfer(escrow.safety_deposit);

        log_escrow_event("CLAIMED", &hashlock_bytes, &caller, escrow.amount);

        main_transfer
            .and(safety_deposit_transfer)
            .then(ext_self::ext(env::current_account_id()).on_escrow_settled(hashlock_bytes))
    }

    pub fn cancel(&mut self, hashlock: Base58CryptoHash) -> Promise {
        let hashlock_bytes: EscrowId = hashlock.into();
        require!(
            self.escrows.contains_key(&hashlock_bytes),
            "Escrow not found"
        );

        let mut escrow = self
            .escrows
            .get(&hashlock_bytes)
            .cloned()
            .expect("Escrow not found");

        require!(!escrow.claimed, "Escrow already claimed");

        let is_public_caller = env::predecessor_account_id() != escrow.taker;

        if escrow.is_source {
            escrow
                .timelocks
                .assert_src_cancellation_window(is_public_caller)
        } else {
            escrow.timelocks.assert_dst_cancellation_window()
        }

        escrow.claimed = true;
        self.escrows.insert(hashlock_bytes, escrow.clone());

        let caller = env::predecessor_account_id();
        let recipient = if escrow.is_source {
            escrow.maker.clone() // Maker gets their funds back
        } else {
            escrow.taker.clone() // Taker gets their funds back
        };

        let main_transfer = match escrow.asset.clone() {
            Asset::Native => Promise::new(recipient).transfer(escrow.amount),
            Asset::Ft(token_id) => ext_fungible_token::ext(token_id)
                .with_static_gas(env::prepaid_gas().saturating_div(4))
                .ft_transfer(
                    recipient,
                    U128(escrow.amount.as_yoctonear()),
                    Some("1inch Fusion+ Swap Cancel".to_string()),
                ),
        };

        let safety_deposit_transfer = Promise::new(caller.clone()).transfer(escrow.safety_deposit);

        log_escrow_event("CANCELED", &hashlock_bytes, &caller, escrow.amount);

        main_transfer
            .and(safety_deposit_transfer)
            .then(ext_self::ext(env::current_account_id()).on_escrow_settled(hashlock_bytes))
    }

    // --- PRIVATE CALLBACKS ---
    #[private]
    pub fn on_ft_pulled_for_escrow(
        &mut self,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
        params: SignedOrder,
        safety_deposit: NearToken,
    ) {
        if result.is_err() {
            log!("FT pull failed. Refunding safety deposit and reverting nonce.");
            Promise::new(params.taker_id).transfer(safety_deposit);
            self.used_nonces.remove(&params.nonce);
            panic_str("Failed to pull FTs from maker; escrow creation aborted.");
        }

        let hashlock_bytes: EscrowId = params.hashlock.into();
        let escrow = Escrow {
            hashlock: hashlock_bytes,
            maker: params.maker_id,
            taker: params.taker_id.clone(),
            asset: Asset::Ft(params.asset_id),
            amount: NearToken::from_yoctonear(params.amount),
            safety_deposit,
            is_source: params.is_source,
            timelocks: Timelocks::new(env::block_timestamp(), params.timelocks),
            claimed: false,
        };
        self.escrows.insert(hashlock_bytes, escrow);
        log_escrow_event(
            "INITIATED_SOURCE",
            &hashlock_bytes,
            &params.taker_id,
            NearToken::from_yoctonear(params.amount),
        );
    }

    #[private]
    pub fn on_escrow_settled(&mut self, hashlock: EscrowId) {
        if let PromiseResult::Successful(_) = env::promise_result(0) {
            // We can clear storage here in the future
            // but I'm not sure about consequences of that
            // let's leave it for now
            // self.escrows.remove(&hashlock);
            env::log_str(&format!(
                "ESCROW_SETTLED: hashlock='{}'",
                bs58::encode(&hashlock).into_string()
            ));
        } else {
            // One or both transfers failed.
            // Revert the `claimed` status to allow another attempt.
            if let Some(mut escrow) = self.escrows.get(&hashlock).cloned() {
                escrow.claimed = false;
                self.escrows.insert(hashlock, escrow);
                env::log_str(&format!(
                    "ESCROW_SETTLEMENT_FAILED: Reverted claimed status for hashlock='{}'",
                    bs58::encode(&hashlock).into_string()
                ));
            }
        }
    }
}

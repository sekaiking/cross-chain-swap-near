use near_sdk::json_types::{Base58CryptoHash, U128};
use near_sdk::store::{IterableMap, IterableSet};
use near_sdk::{
    base64, bs58, env, ext_contract, log, near, require, serde_json, AccountId, NearToken, Promise,
    PromiseOrValue, PromiseResult, PublicKey,
};

// --- Module Declarations ---
mod deposit;
mod escrow;
mod signatures;
mod timelocks;
mod utils;

// --- Use Declarations ---
use crate::deposit::{DepositManager, HasDeposits};
use crate::escrow::{Asset, Escrow, EscrowId, FtMessage};
use crate::signatures::{verify_maker_signature, SignedOrder};
use crate::timelocks::Timelocks;
use crate::utils::log_escrow_event;

// --- External Contract Interfaces ---
#[ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn on_escrow_settled(
        &mut self,
        hashlock: EscrowId,
        maker_id: AccountId,
        taker_id: AccountId,
        is_source: bool,
        is_cancel: bool,
    );
    fn on_deposit_withdrawn(&mut self, account_id: AccountId, token_id: AccountId, amount: U128);
}

// --- Contract State ---
#[near(contract_state)]
pub struct Contract {
    pub owner_id: AccountId,
    pub escrows: IterableMap<EscrowId, Escrow>,
    pub deposits: DepositManager,
    pub used_nonces: IterableSet<u128>,
    pub registered_keys: IterableMap<AccountId, Vec<PublicKey>>,
}

// Define the default, which automatically initializes the contract
impl Default for Contract {
    fn default() -> Self {
        Self {
            owner_id: env::predecessor_account_id(),
            escrows: IterableMap::new(b"e"),
            deposits: DepositManager::new(),
            used_nonces: IterableSet::new(b"u"),
            registered_keys: IterableMap::new(b"k"),
        }
    }
}

// --- Contract Implementation ---
#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            owner_id,
            escrows: IterableMap::new(b"e"),
            deposits: DepositManager::new(),
            used_nonces: IterableSet::new(b"u"),
            registered_keys: IterableMap::new(b"k"),
        }
    }

    #[payable]
    pub fn register_keys(&mut self, public_keys: Vec<PublicKey>) {
        let account_id = env::signer_account_id();
        let mut keys = self
            .registered_keys
            .get(&account_id)
            .cloned()
            .unwrap_or_default();
        for pk in public_keys {
            if !keys.contains(&pk) {
                keys.push(pk.clone());
            }
        }
        self.registered_keys.insert(account_id, keys);
    }

    pub fn get_registered_keys(&self, account_id: AccountId) -> Vec<PublicKey> {
        self.registered_keys
            .get(&account_id)
            .cloned()
            .unwrap_or_default()
    }

    // --- Deposit Management ---
    pub fn withdraw_deposit(&mut self, token_id: AccountId, amount: U128) -> Promise {
        let account_id = env::predecessor_account_id();
        self.deposits
            .assert_available_for_withdrawal(&account_id, &token_id, amount);
        self.deposits.debit_total(&account_id, &token_id, amount);

        ext_fungible_token::ext(token_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(env::prepaid_gas().saturating_div(4))
            .ft_transfer(
                account_id.clone(),
                amount,
                Some("Deposit withdrawal".to_string()),
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(env::prepaid_gas().saturating_div(4))
                    .on_deposit_withdrawn(account_id, token_id, amount),
            )
    }

    pub fn get_available_balance(&self, account_id: AccountId, token_id: AccountId) -> U128 {
        self.deposits.get_available_balance(&account_id, &token_id)
    }

    // --- Core HTLC Logic ---

    /// Primary entry point for all Fungible Token interactions.
    /// Can either be a deposit or the creation of a destination-side escrow.
    #[payable]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let token_contract_id = env::predecessor_account_id();
        let ft_message: FtMessage = serde_json::from_str(&msg).expect("Invalid FtMessage format");

        match ft_message {
            FtMessage::Deposit => {
                self.deposits
                    .credit_total(&sender_id, &token_contract_id, amount);
                log!(
                    "DEPOSIT: account='{}', token='{}', amount='{}'",
                    sender_id,
                    token_contract_id,
                    amount.0
                );
            }
            FtMessage::CreateDestinationEscrow {
                hashlock,
                maker_id,
                timelocks,
            } => {
                let resolver_id = sender_id;
                let safety_deposit = env::attached_deposit();
                require!(
                    safety_deposit.as_yoctonear() > 0,
                    "A native NEAR safety deposit must be attached"
                );

                let hashlock_bytes: EscrowId = hashlock.into();
                require!(
                    !self.escrows.contains_key(&hashlock_bytes),
                    "Escrow already exists"
                );
                timelocks.validate();

                let escrow = Escrow {
                    hashlock: hashlock_bytes,
                    maker: maker_id,
                    taker: resolver_id.clone(),
                    asset: Asset::Ft(token_contract_id),
                    amount: NearToken::from_yoctonear(amount.0),
                    safety_deposit,
                    is_source: false,
                    timelocks: Timelocks::new(env::block_timestamp(), timelocks),
                    claimed: false,
                };
                self.escrows.insert(hashlock_bytes, escrow);
                log_escrow_event(
                    "INITIATED_DESTINATION",
                    &hashlock_bytes,
                    &resolver_id,
                    NearToken::from_yoctonear(amount.0),
                );
            }
        }
        PromiseOrValue::Value(U128(0))
    }

    /// Executed by a Resolver to create a source-side (NEAR -> Other) escrow from a Maker's signed intent.
    #[payable]
    pub fn initiate_source_escrow(
        &mut self,
        params: SignedOrder,
        signature: String,
        public_key: PublicKey,
    ) {
        let resolver_id = env::predecessor_account_id();
        let safety_deposit = env::attached_deposit();
        require!(
            safety_deposit.as_yoctonear() > 0,
            "A native NEAR safety deposit must be attached"
        );

        // Verify signature and order integrity
        let maker_keys = self.get_registered_keys(params.maker_id.clone());
        require!(
            maker_keys.contains(&public_key),
            "Public key not registered for maker"
        );
        let signature_bytes = base64::decode(&signature).expect("Invalid signature format");
        verify_maker_signature(
            &params,
            &signature_bytes,
            &public_key,
            &mut self.used_nonces,
        );
        params.timelocks.validate();

        // Verify maker has sufficient available funds
        let amount_u128 = params.amount;
        self.deposits
            .assert_available_for_escrow(&params.maker_id, &params.asset_id, amount_u128);

        // Lock the funds in the maker's internal ledger
        self.deposits
            .credit_locked(&params.maker_id, &params.asset_id, amount_u128);

        // Create the escrow
        let hashlock_bytes: EscrowId = params.hashlock.into();
        let escrow = Escrow {
            hashlock: hashlock_bytes,
            maker: params.maker_id,
            taker: resolver_id.clone(),
            asset: Asset::Ft(params.asset_id),
            amount: NearToken::from_yoctonear(params.amount.0),
            safety_deposit,
            is_source: true,
            timelocks: Timelocks::new(env::block_timestamp(), params.timelocks),
            claimed: false,
        };
        self.escrows.insert(hashlock_bytes, escrow);
        log_escrow_event(
            "INITIATED_SOURCE",
            &hashlock_bytes,
            &resolver_id,
            NearToken::from_yoctonear(params.amount.0),
        );
    }

    /// Claims the funds from an escrow by revealing the secret.
    pub fn withdraw(&mut self, secret: String) -> Promise {
        let secret_bytes = base64::decode(secret).expect("Invalid base64 secret");
        let hashlock_bytes: EscrowId = env::sha256_array(&secret_bytes);

        let escrow = self
            .escrows
            .get(&hashlock_bytes)
            .cloned()
            .expect("Escrow not found");
        require!(!escrow.claimed, "Escrow already claimed");

        // Validate timelocks
        let is_public_caller = env::predecessor_account_id() != escrow.taker;
        if escrow.is_source {
            escrow
                .timelocks
                .assert_src_withdrawal_window(is_public_caller);
        } else {
            escrow
                .timelocks
                .assert_dst_withdrawal_window(is_public_caller);
        }

        // Update escrow as claimed
        let mut updated_escrow = escrow.clone();
        updated_escrow.claimed = true;
        self.escrows.insert(hashlock_bytes, updated_escrow);

        let caller = env::predecessor_account_id();
        let (recipient, asset_token_id) = if escrow.is_source {
            // Source (NEAR->Other): Taker/Resolver claims the NEAR funds
            (escrow.taker.clone(), escrow.asset.ft_token_id())
        } else {
            // Destination (Other->NEAR): Maker claims the NEAR funds
            (escrow.maker.clone(), escrow.asset.ft_token_id())
        };

        let main_transfer = ext_fungible_token::ext(asset_token_id)
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .ft_transfer(
                recipient,
                U128(escrow.amount.as_yoctonear()),
                Some("1inch Fusion+ Swap".to_string()),
            );

        let safety_deposit_transfer = Promise::new(caller.clone()).transfer(escrow.safety_deposit);

        log_escrow_event("CLAIMED", &hashlock_bytes, &caller, escrow.amount);

        main_transfer.and(safety_deposit_transfer).then(
            ext_self::ext(env::current_account_id()).on_escrow_settled(
                hashlock_bytes,
                escrow.maker,
                escrow.taker,
                escrow.is_source,
                false,
            ),
        )
    }

    /// Cancels an expired escrow, returning funds to the original depositor.
    pub fn cancel(&mut self, hashlock: Base58CryptoHash) -> Promise {
        let hashlock_bytes: EscrowId = hashlock.into();
        let escrow = self
            .escrows
            .get(&hashlock_bytes)
            .cloned()
            .expect("Escrow not found");
        require!(!escrow.claimed, "Escrow already claimed");

        // Validate timelocks
        let is_public_caller = env::predecessor_account_id() != escrow.taker;
        if escrow.is_source {
            escrow
                .timelocks
                .assert_src_cancellation_window(is_public_caller)
        } else {
            escrow.timelocks.assert_dst_cancellation_window()
        }

        // Update escrow as claimed
        let mut updated_escrow = escrow.clone();
        updated_escrow.claimed = true;
        self.escrows.insert(hashlock_bytes, updated_escrow);

        let caller = env::predecessor_account_id();
        let main_promise = if escrow.is_source {
            // Source (NEAR->Other): Refund is internal. Just update the ledger. No transfer.
            // The ledger update happens in `on_escrow_settled`.
            Promise::new(env::current_account_id())
        } else {
            // Destination (Other->NEAR): Taker/Resolver gets their funds back.
            ext_fungible_token::ext(escrow.asset.ft_token_id())
                .with_attached_deposit(NearToken::from_yoctonear(1))
                .ft_transfer(
                    escrow.taker.clone(),
                    U128(escrow.amount.as_yoctonear()),
                    Some("1inch Fusion+ Cancel".to_string()),
                )
        };

        let safety_deposit_transfer = Promise::new(caller.clone()).transfer(escrow.safety_deposit);
        log_escrow_event("CANCELED", &hashlock_bytes, &caller, escrow.amount);

        main_promise.and(safety_deposit_transfer).then(
            ext_self::ext(env::current_account_id()).on_escrow_settled(
                hashlock_bytes,
                escrow.maker,
                escrow.taker,
                escrow.is_source,
                true,
            ),
        )
    }

    // --- PRIVATE CALLBACKS ---
    #[private]
    pub fn on_escrow_settled(
        &mut self,
        hashlock: EscrowId,
        maker_id: AccountId,
        taker_id: AccountId,
        is_source: bool,
        is_cancel: bool,
    ) {
        let escrow = self
            .escrows
            .get(&hashlock)
            .cloned()
            .expect("Escrow not found in callback");

        if let PromiseResult::Successful(_) = env::promise_result(0) {
            if is_source {
                let amount = U128(escrow.amount.as_yoctonear());
                let token_id = escrow.asset.ft_token_id();
                if is_cancel {
                    // Source cancellation: funds returned to maker's available pool.
                    self.deposits.debit_locked(&maker_id, &token_id, amount);
                } else {
                    // Source successful claim: funds are gone. Debit both ledgers.
                    self.deposits.debit_locked(&maker_id, &token_id, amount);
                    self.deposits.debit_total(&maker_id, &token_id, amount);
                }
            }
            // For destination escrows, no ledger update is needed as funds were never in the internal ledger.
            log!(
                "ESCROW_SETTLED: hashlock='{}'",
                bs58::encode(&hashlock).into_string()
            );
        } else {
            // A transfer failed. Revert the `claimed` status to allow another attempt.
            if let Some(mut escrow) = self.escrows.get(&hashlock).cloned() {
                escrow.claimed = false;
                self.escrows.insert(hashlock, escrow);
                log!(
                    "ESCROW_SETTLEMENT_FAILED: Reverted claimed status for hashlock='{}'",
                    bs58::encode(&hashlock).into_string()
                );
            }
        }
    }

    #[private]
    pub fn on_deposit_withdrawn(
        &mut self,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
        account_id: AccountId,
        token_id: AccountId,
        amount: U128,
    ) {
        if result.is_err() {
            // Transfer failed, credit the funds back to the user's deposit balance
            self.deposits.credit_total(&account_id, &token_id, amount);
            log!(
                "WITHDRAWAL_FAILED: Reverted deposit for account='{}', token='{}', amount='{}'",
                account_id,
                token_id,
                amount.0
            );
        }
    }
}

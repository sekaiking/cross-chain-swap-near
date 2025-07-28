use near_sdk::json_types::{Base58CryptoHash, U128};
use near_sdk::store::IterableMap;
use near_sdk::{
    base64, bs58, env, near, require, serde_json, AccountId, NearToken, Promise, PromiseOrValue,
    PromiseResult,
};

mod escrow;
mod timelocks;
mod utils;

use escrow::{Asset, Escrow, EscrowId, FtOnTransferMsg};
use timelocks::{TimelockDelays, Timelocks};
use utils::log_escrow_event;

// External contract interfaces
#[near_sdk::ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[near_sdk::ext_contract(ext_self)]
pub trait SelfCallbacks {
    fn on_escrow_settled(&mut self, hashlock: EscrowId);
}

// Define the contract structure
#[near(contract_state)]
pub struct Contract {
    pub owner_id: AccountId,
    // The main collection storing all active escrows, keyed by their EscrowId (secret_hash)
    pub escrows: IterableMap<EscrowId, Escrow>,
}

// Define the default, which automatically initializes the contract
impl Default for Contract {
    fn default() -> Self {
        Self {
            owner_id: env::predecessor_account_id(),
            escrows: IterableMap::new(b"e"),
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
        }
    }

    /// Initiates a escrow with native NEAR.
    #[payable]
    pub fn initiate_escrow(
        &mut self,
        hashlock: Base58CryptoHash,
        taker: AccountId,
        timelocks: TimelockDelays,
        safety_deposit: NearToken,
        is_source: bool,
    ) {
        // 1. Validate the timelock configuration first.
        timelocks.validate();

        // 2. Derive the swap amount from the attached deposit.
        let attached_deposit = env::attached_deposit();
        let amount = attached_deposit.saturating_sub(safety_deposit);
        require!(
            amount.as_yoctonear() > 0,
            "Deposit must be greater than safety_deposit"
        );

        // 3. Check for hashlock collision.
        let hashlock_bytes: EscrowId = hashlock.into();
        require!(
            !self.escrows.contains_key(&hashlock_bytes),
            "Escrow with this hashlock already exists"
        );

        // 4. Construct the Escrow object.
        let maker = env::predecessor_account_id();
        let escrow = Escrow {
            hashlock: hashlock_bytes,
            maker: maker.clone(),
            taker,
            asset: Asset::Native, // This function ONLY handles Native NEAR.
            amount,
            safety_deposit,
            is_source,
            timelocks: Timelocks::new(env::block_timestamp(), timelocks),
            claimed: false,
        };

        // 5. Save the escrow.
        self.escrows.insert(hashlock_bytes, escrow);

        log_escrow_event("INITIATED_NATIVE", &hashlock_bytes, &maker, amount);
    }

    /// NEP-141 Receiver: Initiates an escrow with a Fungible Token.
    ///
    /// To escrow a Fungible Token (like wNEAR or USDC), the user (or resolver script acting on their behalf)
    /// does not call the contract directly. Instead, they execute a single transaction by calling
    /// the ft_transfer_call function on the token contract itself.
    ///
    /// This function is called by a token contract when a user executes `ft_transfer_call`.
    /// The `safety_deposit` MUST be attached to the `ft_transfer_call` as native NEAR.
    ///
    /// # Arguments
    /// - `sender_id`: The user who initiated the transfer. This is a trusted arg from the token contract.
    /// - `amount`: The amount of FTs transferred. This is a trusted arg.
    /// - `msg`: A JSON-serialized `FtOnTransferMsg` containing the escrow parameters.
    ///
    /// # Returns
    /// A `PromiseOrValue` that returns `U128(0)` to signify we are consuming all transferred tokens.
    /// If the logic panics, the token contract will automatically refund the FTs to the user.
    #[payable]
    pub fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // 1. Get the token contract that called this function (the asset being transferred).
        let token_contract_id = env::predecessor_account_id();

        // 2. Deserialize the message from the user.
        let ft_msg: FtOnTransferMsg =
            serde_json::from_str(&msg).expect("Invalid FtOnTransferMsg format");

        // 3. Validate the timelock configuration.
        ft_msg.timelocks.validate();

        // 4. Verify the native NEAR safety deposit attached to the call.
        let safety_deposit = env::attached_deposit();
        require!(
            safety_deposit.as_yoctonear() > 0,
            "A native NEAR safety deposit must be attached"
        );

        // 5. Check for hashlock collision.
        let hashlock_bytes: EscrowId = ft_msg.hashlock.into();
        require!(
            !self.escrows.contains_key(&hashlock_bytes),
            "Escrow with this hashlock already exists"
        );

        // 6. Construct the Escrow object.
        let escrow = Escrow {
            hashlock: hashlock_bytes,
            maker: sender_id.clone(), // The user is the `maker`.
            taker: ft_msg.taker,
            asset: Asset::Ft(token_contract_id), // The asset is the contract that called us.
            amount: NearToken::from_yoctonear(amount.0),
            safety_deposit,
            is_source: ft_msg.is_source,
            timelocks: Timelocks::new(env::block_timestamp(), ft_msg.timelocks),
            claimed: false,
        };

        // 7. Save the escrow to state.
        self.escrows.insert(hashlock_bytes, escrow);

        log_escrow_event(
            "INITIATED_FT",
            &hashlock_bytes,
            &sender_id,
            NearToken::from_yoctonear(amount.0),
        );

        // 8. Return U128(0) to indicate we've consumed the tokens and aren't returning any.
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
            escrow.taker.clone() // Source: Taker/Resolver claims
        } else {
            escrow.maker.clone() // Destination: Maker/User claims
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
            escrow.maker.clone() // Source: Maker/User gets their funds back
        } else {
            escrow.taker.clone() // Destination: Taker/Resolver gets their funds back
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
    pub fn on_escrow_settled(&mut self, hashlock: EscrowId) {
        if let PromiseResult::Successful(_) = env::promise_result(0) {
            // Both transfers succeeded, we can safely remove the escrow.
            self.escrows.remove(&hashlock);
            env::log_str(&format!(
                "ESCROW_CLEANUP_SUCCESS: hashlock='{}'",
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

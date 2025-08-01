use near_sdk::{json_types::U128, near, require, store::IterableMap, AccountId};

#[near(serializers = [borsh])]
pub struct DepositManager {
    // AccountId -> TokenId -> Balance
    pub deposits: IterableMap<AccountId, IterableMap<AccountId, U128>>,
    pub locked_deposits: IterableMap<AccountId, IterableMap<AccountId, U128>>,
}

impl DepositManager {
    pub fn new() -> Self {
        Self {
            deposits: IterableMap::new(b"d"),
            locked_deposits: IterableMap::new(b"l"),
        }
    }

    pub fn get_all_user_deposits(&self, account_id: &AccountId) -> Vec<(AccountId, U128)> {
        self.deposits
            .get(account_id)
            .map(|user_deposits| {
                user_deposits
                    .iter()
                    .map(|(token_id, balance)| (token_id.clone(), balance.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_all_users_with_deposits(&self) -> Vec<AccountId> {
        self.deposits.keys().cloned().collect()
    }
}

pub trait HasDeposits {
    fn get_total_balance(&self, account_id: &AccountId, token_id: &AccountId) -> U128;
    fn get_locked_balance(&self, account_id: &AccountId, token_id: &AccountId) -> U128;
    fn get_available_balance(&self, account_id: &AccountId, token_id: &AccountId) -> U128;
    fn credit_total(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128);
    fn debit_total(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128);
    fn credit_locked(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128);
    fn debit_locked(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128);
    fn assert_available_for_escrow(
        &self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: U128,
    );
    fn assert_available_for_withdrawal(
        &self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: U128,
    );
}

impl HasDeposits for DepositManager {
    fn get_total_balance(&self, account_id: &AccountId, token_id: &AccountId) -> U128 {
        self.deposits
            .get(account_id)
            .and_then(|m| m.get(token_id))
            .cloned()
            .unwrap_or(U128(0))
    }

    fn get_locked_balance(&self, account_id: &AccountId, token_id: &AccountId) -> U128 {
        self.locked_deposits
            .get(account_id)
            .and_then(|m| m.get(token_id))
            .cloned()
            .unwrap_or(U128(0))
    }

    fn get_available_balance(&self, account_id: &AccountId, token_id: &AccountId) -> U128 {
        let total = self.get_total_balance(account_id, token_id).0;
        let locked = self.get_locked_balance(account_id, token_id).0;
        U128(total.saturating_sub(locked))
    }

    fn credit_total(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128) {
        if !self.deposits.contains_key(account_id) {
            self.deposits
                .insert(account_id.clone(), IterableMap::new(b"s"));
        }
        let user_deposits = self.deposits.get_mut(account_id).unwrap();
        let current_balance = user_deposits.get(token_id).unwrap_or(&U128(0)).0;
        user_deposits.insert(token_id.clone(), U128(current_balance + amount.0));
    }

    fn debit_total(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128) {
        let user_deposits = self
            .deposits
            .get_mut(account_id)
            .expect("No deposits for this user");
        let current_balance = user_deposits.get(token_id).unwrap_or(&U128(0)).0;
        user_deposits.insert(
            token_id.clone(),
            U128(current_balance.saturating_sub(amount.0)),
        );
    }

    fn credit_locked(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128) {
        if !self.locked_deposits.contains_key(account_id) {
            self.locked_deposits
                .insert(account_id.clone(), IterableMap::new(b"x"));
        }
        let user_locked = self.locked_deposits.get_mut(account_id).unwrap();
        let current_locked = user_locked.get(token_id).unwrap_or(&U128(0)).0;
        user_locked.insert(token_id.clone(), U128(current_locked + amount.0));
    }

    fn debit_locked(&mut self, account_id: &AccountId, token_id: &AccountId, amount: U128) {
        let user_locked = self
            .locked_deposits
            .get_mut(account_id)
            .expect("No locked deposits for this user");
        let current_locked = user_locked.get(token_id).unwrap_or(&U128(0)).0;
        user_locked.insert(
            token_id.clone(),
            U128(current_locked.saturating_sub(amount.0)),
        );
    }

    fn assert_available_for_escrow(
        &self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: U128,
    ) {
        let available = self.get_available_balance(account_id, token_id);
        require!(
            available.0 >= amount.0,
            "Insufficient available funds for escrow"
        );
    }

    fn assert_available_for_withdrawal(
        &self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: U128,
    ) {
        let available = self.get_available_balance(account_id, token_id);
        require!(amount.0 > 0, "Withdrawal amount must be positive");
        require!(
            available.0 >= amount.0,
            "Insufficient available funds for withdrawal"
        );
    }
}

impl Default for DepositManager {
    fn default() -> Self {
        Self::new()
    }
}

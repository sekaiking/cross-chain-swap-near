use near_sdk::near;
use near_sdk::store::IterableMap;
use near_sdk::AccountId;
use near_sdk::NearToken;
use swap::Swap;
use swap::SwapId;

mod swap;


// Define the contract structure
#[near(contract_state)]
pub struct Contract {
    // The main collection storing all active swaps, keyed by their SwapId (secret_hash)
    pub swaps: IterableMap<SwapId, Swap>,
    // Store accidentally sent funds. Key is the token_id ("" for native), value is the balance.
    pub rescued_funds: IterableMap<AccountId, NearToken>,
}

// Define the default, which automatically initializes the contract
impl Default for Contract {
    fn default() -> Self {
        Self {
            swaps: IterableMap::new(b"s"),
            rescued_funds: IterableMap::new(b"f")
        }
    }
}

// Implement the contract structure
#[near]
impl Contract {
}

/*
 * The rest of this file holds the inline tests for the code above
 * Learn more about Rust tests: https://doc.rust-lang.org/book/ch11-01-writing-tests.html
 */
#[cfg(test)]
mod tests {
    // use super::*;
    //
    // #[test]
    // fn get_default_greeting() {
    //     let contract = Contract::default();
    //     // this test did not call set_greeting so should return the default "Hello" greeting
    //     assert_eq!(contract.get_greeting(), "Hello");
    // }
    //
    // #[test]
    // fn set_then_get_greeting() {
    //     let mut contract = Contract::default();
    //     contract.set_greeting("howdy".to_string());
    //     assert_eq!(contract.get_greeting(), "howdy");
    // }
}

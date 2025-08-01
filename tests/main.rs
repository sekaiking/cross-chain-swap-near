use anyhow::Result;
use near_workspaces::network::Sandbox;
use near_workspaces::types::SecretKey;
use near_workspaces::{prelude::*, Account, Contract, DevNetwork, Worker};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Duration;

// Your contract's structs need to be accessible in the test environment.
// Make sure they are public in your contract code.
use cross_chain_swap_near::{SignedOrder, TimelockDelays, FtOnTransferMsg};

const FT_WASM_PATH: &str = "./path/to/a/mock_ft_contract.wasm"; // IMPORTANT: Provide a path to a generic FT wasm
const HTLC_WASM_PATH: &str = "./target/wasm32-unknown-unknown/release/htlc_contract.wasm";

/// Helper function to set up the testing environment.
/// This will:
/// 1. Initialize a sandbox environment.
/// 2. Deploy the HTLC and a mock Fungible Token contract.
/// 3. Create accounts for the Maker and Resolver.
/// 4. Mint some FTs for the Maker and Resolver.
async fn setup() -> Result<(
    Worker<Sandbox>,
    Contract,
    Contract,
    Account,
    Account,
)> {
    let worker = near_workspaces::sandbox().await?;
    let htlc_wasm = std::fs::read(HTLC_WASM_PATH)?;
    let ft_wasm = std::fs::read(FT_WASM_PATH).expect("You must provide a valid path to a fungible token WASM file. You can get one from the near-examples repo.");


    // Deploy Contracts
    let htlc_contract = worker.dev_deploy(&htlc_wasm).await?;
    let ft_contract = worker.dev_deploy(&ft_wasm).await?;

    // Initialize FT Contract
    ft_contract
        .call("new_default_meta")
        .args_json(json!({ "owner_id": htlc_contract.id(), "total_supply": "1000000000000000000000000000" }))
        .transact()
        .await?
        .into_result()?;

    // Create user accounts
    let maker = worker.dev_create_account().await?;
    let resolver = worker.dev_create_account().await?;

    // Pre-fund users with FTs
    let storage_deposit = near_sdk::NearToken::from_yoctonear(1250000000000000000000);
    for user in [&maker, &resolver] {
        ft_contract
            .call("storage_deposit")
            .args_json(json!({ "account_id": user.id() }))
            .deposit(storage_deposit)
            .transact()
            .await?
            .into_result()?;

        ft_contract
            .call("ft_transfer")
            .args_json(json!({ "receiver_id": user.id(), "amount": "1000000000000000000000" })) // 1000 FT
            .deposit(near_sdk::NearToken::from_yoctonear(1))
            .transact()
            .await?
            .into_result()?;
    }

    Ok((worker, htlc_contract, ft_contract, maker, resolver))
}

#[tokio::test]
async fn test_full_source_escrow_flow() -> Result<()> {
    // 1. ARRANGE: Setup contracts, users, and necessary pre-conditions.
    let (_worker, htlc_contract, ft_contract, maker, resolver) = setup().await?;

    // Maker registers their public key.
    maker
        .call(htlc_contract.id(), "register_key")
        .transact()
        .await?
        .into_result()?;

    // Maker approves the HTLC to spend their FTs.
    maker
        .call(ft_contract.id(), "ft_approve")
        .args_json(json!({
            "contract_id": htlc_contract.id(),
            "amount": "100000000000000000000" // 100 wNEAR
        }))
        .deposit(near_sdk::NearToken::from_yoctonear(1))
        .transact()
        .await?
        .into_result()?;

    // Generate secret and hashlock off-chain.
    let secret = "my super secret string".as_bytes();
    let hashlock = Sha256::digest(secret).to_vec();

    // Maker creates and signs an order off-chain.
    let params = SignedOrder {
        nonce: 1,
        maker_id: maker.id().clone(),
        taker_id: resolver.id().clone(),
        asset_id: ft_contract.id().clone(),
        amount: 100_000_000_000_000_000_000, // 100 wNEAR
        hashlock: hashlock.clone().try_into().unwrap(),
        timelocks: TimelockDelays {
            src_withdrawal_delay: 0,
            src_public_withdrawal_delay: 300,
            src_cancellation_delay: 600,
            src_public_cancellation_delay: 900,
            dst_withdrawal_delay: 0,
            dst_public_withdrawal_delay: 120,
            dst_cancellation_delay: 240,
        },
        is_source: true,
    };

    let message = params.to_message_bytes();
    let secret_key = maker.secret_key().to_string().parse().unwrap();
    let signature = match (secret_key.sign(&message), secret_key.public_key()) {
        (near_crypto::Signature::ED25519(sig), near_crypto::PublicKey::ED25519(pk)) => {
            SignedNep413Payload {
                params,
                public_key: pk.0,
                signature: sig.to_bytes(),
            }
        }
    }

    // 2. ACT: Resolver initiates the escrow on-chain.
    let result = resolver
        .call(htlc_contract.id(), "initiate_source_escrow")
        .args_json(json!({
            "params": params,
            "signature": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature.as_bytes()),
            "public_key": maker.secret_key().public_key().to_string()
        }))
        .deposit(near_sdk::NearToken::from_millinear(100)) // 0.1 NEAR safety deposit
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    
    println!("Initiate Source Escrow logs: {:?}", result.logs());
    assert!(result.is_success());

    // 3. ASSERT: Check that tokens were pulled into the contract.
    let htlc_balance: String = ft_contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": htlc_contract.id() }))
        .await?
        .json()?;
    assert_eq!(htlc_balance, "100000000000000000000"); // 100 wNEAR

    // 4. ACT (Part 2): Resolver reveals secret to withdraw funds.
    let withdraw_result = resolver
        .call(htlc_contract.id(), "withdraw")
        .args_json(json!({ "secret": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, secret) }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;

    println!("Withdraw logs: {:?}", withdraw_result.logs());
    assert!(withdraw_result.is_success());

    // 5. ASSERT (Part 2): Check that resolver received the funds.
    let resolver_balance: String = ft_contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": resolver.id() }))
        .await?
        .json()?;
    assert_eq!(resolver_balance, "1100000000000000000000"); // 1000 initial + 100 from swap

    Ok(())
}

#[tokio::test]
async fn test_full_destination_escrow_flow() -> Result<()> {
    // 1. ARRANGE: Setup contracts, users, and necessary pre-conditions.
    let (_worker, htlc_contract, ft_contract, maker, resolver) = setup().await?;

    // Generate secret and hashlock off-chain.
    let secret = "another secret for destination".as_bytes();
    let hashlock = Sha256::digest(secret).to_vec();

    let msg_payload = FtOnTransferMsg {
        hashlock: hashlock.clone().try_into().unwrap(),
        maker_id: maker.id().clone(),
        timelocks: TimelockDelays {
            // These delays would typically be different from a source escrow
            src_withdrawal_delay: 600,
            src_public_withdrawal_delay: 900,
            src_cancellation_delay: 1200,
            src_public_cancellation_delay: 1500,
            dst_withdrawal_delay: 0,
            dst_public_withdrawal_delay: 300,
            dst_cancellation_delay: 600,
        },
    };

    // 2. ACT: Resolver initiates the escrow via ft_transfer_call.
    let result = resolver
        .call(ft_contract.id(), "ft_transfer_call")
        .args_json(json!({
            "receiver_id": htlc_contract.id(),
            "amount": "50000000000000000000", // 50 wNEAR
            "msg": serde_json::to_string(&msg_payload)?
        }))
        .deposit(near_sdk::NearToken::from_yoctonear(1)) // for ft_transfer_call
        .gas(near_sdk::Gas::from_tgas(100))
        .transact_with_solution(async |tx, _network, _rpc_client| {
            // Manually attach the safety deposit since it's part of the same transaction
            tx.actions(vec![near_workspaces::types::Action::Transfer { deposit: near_sdk::NearToken::from_millinear(100) }])
        })
        .await?
        .into_result()?;

    println!("Initiate Destination Escrow logs: {:?}", result.logs());
    assert!(result.is_success());

    // 3. ASSERT: Check that tokens were transferred into the contract.
    let htlc_balance: String = ft_contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": htlc_contract.id() }))
        .await?
        .json()?;
    assert_eq!(htlc_balance, "50000000000000000000"); // 50 wNEAR

    // 4. ACT (Part 2): Maker reveals secret to withdraw funds.
    let withdraw_result = maker
        .call(htlc_contract.id(), "withdraw")
        .args_json(json!({ "secret": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, secret) }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    assert!(withdraw_result.is_success());

    // 5. ASSERT (Part 2): Check that maker received the funds.
    let maker_balance: String = ft_contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": maker.id() }))
        .await?
        .json()?;
    assert_eq!(maker_balance, "1050000000000000000000"); // 1000 initial + 50 from swap

    Ok(())
}

#[tokio::test]
async fn test_source_escrow_cancellation() -> Result<()> {
    // 1. ARRANGE: Setup an escrow with a short cancellation window.
    let (worker, htlc_contract, ft_contract, maker, resolver) = setup().await?;

    maker.call(htlc_contract.id(), "register_key").transact().await?.into_result()?;
    maker.call(ft_contract.id(), "ft_approve")
        .args_json(json!({ "contract_id": htlc_contract.id(), "amount": "100000000000000000000" }))
        .deposit(near_sdk::NearToken::from_yoctonear(1))
        .transact().await?.into_result()?;

    let hashlock = Sha256::digest(b"cancellable").to_vec();
    let params = SignedOrder {
        nonce: 1,
        maker_id: maker.id().clone(),
        taker_id: resolver.id().clone(),
        asset_id: ft_contract.id().clone(),
        amount: 100_000_000_000_000_000_000,
        hashlock: hashlock.clone().try_into().unwrap(),
        timelocks: TimelockDelays {
            src_cancellation_delay: 2, // Cancelable after 2 seconds
            src_withdrawal_delay: 1,
            src_public_withdrawal_delay: 1,
            src_public_cancellation_delay: 3,
            dst_cancellation_delay: 10,
            dst_withdrawal_delay: 1,
            dst_public_withdrawal_delay: 1,
        },
        is_source: true,
    };
    let signature = maker.sign(¶ms.to_message_bytes());

    // Initiate the escrow
    resolver.call(htlc_contract.id(), "initiate_source_escrow")
        .args_json(json!({
            "params": params,
            "signature": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature.as_bytes()),
            "public_key": maker.secret_key().public_key().to_string()
        }))
        .deposit(near_sdk::NearToken::from_millinear(100))
        .max_gas().transact().await?.into_result()?;

    // 2. ACT: Fast-forward time past the cancellation delay and cancel.
    worker.fast_forward(Duration::from_secs(3)).await?;

    let maker_initial_ft_balance: String = ft_contract.view("ft_balance_of").args_json(json!({"account_id": maker.id()})).await?.json()?;
    assert_eq!(maker_initial_ft_balance, "900000000000000000000"); // 1000 - 100

    let result = resolver // The original taker can cancel
        .call(htlc_contract.id(), "cancel")
        .args_json(json!({ "hashlock": hashlock.try_into().unwrap() }))
        .max_gas()
        .transact()
        .await?
        .into_result()?;
    
    println!("Cancellation logs: {:?}", result.logs());
    assert!(result.is_success());

    // 3. ASSERT: Check that funds were refunded to the maker.
    let maker_final_ft_balance: String = ft_contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": maker.id() }))
        .await?
        .json()?;
    assert_eq!(maker_final_ft_balance, "1000000000000000000000"); // Back to 1000

    let htlc_final_balance: String = ft_contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": htlc_contract.id() }))
        .await?
        .json()?;
    assert_eq!(htlc_final_balance, "0");

    Ok(())
}

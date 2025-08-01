// resolver.js

import { connect, keyStores, KeyPair } from "near-api-js";
import { ethers } from "ethers";
import dotenv from "dotenv";
import bs58 from "bs58";

dotenv.config();

// =================================================================================
// PASTE THE OUTPUT FROM YOUR FRONTEND HERE
// =================================================================================
const resolverPayload = {
  "params": {
    "maker_id": "fusss.testnet",
    "asset_id": "wrap.testnet",
    "amount": "10000000000000000000000",
    "hashlock": "B9YjHgh3TD7azGpZ4RCUMxjRcAe5GawfZDctuX3ujzUw",
    "timelocks": {
      "src_withdrawal_delay": 3600,
      "src_public_withdrawal_delay": 7200,
      "src_cancellation_delay": 14400,
      "src_public_cancellation_delay": 28800,
      "dst_withdrawal_delay": 1800,
      "dst_public_withdrawal_delay": 3600,
      "dst_cancellation_delay": 10800
    },
    "nonce": 1753980045623
  },
  "signature": "yZ7qquLUjQIZLK+Ajtlg0rJy84VkSpFL9Sw3Vzh1nKJVGMEtDv7cgpSg2Oc3s47iDS2QHgbQdEfPBnpyFGiYBw==",
  "public_key": "ed25519:6KMcnToP7fvnWMU2HRohK9kDjCGBxkxK4P6fBWU6DizF"
};

// You get this from the toast message in the UI
const secret_base64 = "i8Sa6+rfL56Iwj0jqbpOP9KpXS4Dhaam4mEJrxnLvjg=";

// The MAKER's Ethereum address where they will receive the WETH
const MAKER_ETH_ADDRESS = "0x...MAKER_ETH_WALLET...";

// Amount of WETH the resolver is giving in exchange for the NEAR tokens
// For the hackathon, we can hardcode this.
const ETH_AMOUNT_TO_SEND = ethers.parseEther("0.0015");

// =================================================================================

// --- Helper function to log steps ---
const logStep = (step, msg) => console.log(`\n[STEP ${step}] ${msg}`);
const logInfo = (msg) => console.log(`  > ${msg}`);

async function main() {
  // --- INITIALIZE CONNECTIONS ---
  logStep(0, "Initializing connections and wallets...");

  // NEAR Connection
  const keyStore = new keyStores.InMemoryKeyStore();
  const keyPair = KeyPair.fromString(process.env.RESOLVER_NEAR_PRIVATE_KEY);
  await keyStore.setKey("testnet", process.env.RESOLVER_NEAR_ACCOUNT_ID, keyPair);
  const near = await connect({
    networkId: "testnet",
    nodeUrl: process.env.NEAR_RPC_URL,
    keyStore,
  });
  const resolverNearAccount = await near.account(process.env.RESOLVER_NEAR_ACCOUNT_ID);
  const nearHtlcContract = {
    accountId: process.env.NEAR_HTLC_CONTRACT_ID,
    viewMethods: [],
    changeMethods: ["initiate_source_escrow", "withdraw"],
  };
  logInfo(`NEAR Resolver: ${resolverNearAccount.accountId}`);

  // Ethereum Connection
  const provider = new ethers.JsonRpcProvider(process.env.ETH_RPC_URL);
  const resolverEthWallet = new ethers.Wallet(process.env.RESOLVER_ETH_PRIVATE_KEY, provider);
  const ethHtlcAbi = [
    "function newSwap(bytes32 _hashlock, address _recipient, address _token, uint256 _amount, uint256 _timelockSeconds) external",
    "function claim(bytes32 _secret) external",
    "event NewSwap(bytes32 indexed hashlock)",
  ];
  const ethHtlcContract = new ethers.Contract(process.env.ETH_HTLC_CONTRACT_ADDRESS, ethHtlcAbi, resolverEthWallet);
  const wethContract = new ethers.Contract(process.env.WETH_TOKEN_ADDRESS_SEPOLIA, ["function approve(address spender, uint256 amount) public returns (bool)"], resolverEthWallet);
  logInfo(`ETH Resolver: ${resolverEthWallet.address}`);

  // --- STEP 1: INITIATE SOURCE ESCROW ON NEAR ---
  logStep(1, "Calling `initiate_source_escrow` on NEAR to lock maker's funds...");
  try {
    const nearTx = await resolverNearAccount.functionCall({
      contractId: nearHtlcContract.accountId,
      methodName: "initiate_source_escrow",
      args: resolverPayload,
      gas: "100000000000000", // 100 TGas
      attachedDeposit: "100000000000000000000000" // 0.1 NEAR safety deposit
    });
    logInfo(`NEAR Tx Hash: https://testnet.nearblocks.io/txns/${nearTx.transaction.hash}`);
    logInfo("Maker's wNEAR is now locked in the NEAR HTLC.");
  } catch (e) {
    console.error("Failed to initiate escrow on NEAR:", e);
    return;
  }

  // --- STEP 2: LOCK RESOLVER'S FUNDS ON ETHEREUM ---
  logStep(2, "Locking resolver's WETH in the Ethereum HTLC...");
  const hashlockBytes = `0x${Buffer.from(bs58.decode(resolverPayload.params.hashlock)).toString('hex')}`;
  logInfo(`Using Hashlock: ${hashlockBytes}`);

  try {
    // Approve the HTLC contract to spend the WETH
    logInfo(`Approving WETH spend for ${ethers.formatEther(ETH_AMOUNT_TO_SEND)} WETH...`);
    const approveTx = await wethContract.approve(process.env.ETH_HTLC_CONTRACT_ADDRESS, ETH_AMOUNT_TO_SEND);
    await approveTx.wait();
    logInfo(`Approve Tx: https://sepolia.etherscan.io/tx/${approveTx.hash}`);

    // Create the swap
    logInfo("Calling `newSwap` on Ethereum HTLC...");
    // IMPORTANT: The timelock here (dst_withdrawal_delay) MUST be shorter than the cancellation
    // timelock on NEAR (src_cancellation_delay) to give the maker time to claim.
    const timelockSeconds = resolverPayload.params.timelocks.dst_withdrawal_delay;
    const ethTx = await ethHtlcContract.newSwap(
      hashlockBytes,
      MAKER_ETH_ADDRESS,
      process.env.WETH_TOKEN_ADDRESS_SEPOLIA,
      ETH_AMOUNT_TO_SEND,
      timelockSeconds
    );
    await ethTx.wait();
    logInfo(`ETH HTLC Tx: https://sepolia.etherscan.io/tx/${ethTx.hash}`);
    logInfo("Resolver's WETH is now locked. The maker can claim it.");
  } catch (e) {
    console.error("Failed to lock funds on Ethereum:", e);
    // In a real scenario, the resolver would now cancel the NEAR escrow to get their safety deposit back.
    return;
  }

  // --- STEP 3: (SIMULATION) MAKER CLAIMS ON ETHEREUM ---
  logStep("3 (SIM)", "Simulating maker's claim on Ethereum to reveal the secret...");
  const secretBytes = `0x${Buffer.from(secret_base64, 'base64').toString('hex')}`;
  logInfo(`Revealing secret: ${secretBytes}`);
  try {
    const claimTx = await ethHtlcContract.claim(secretBytes);
    await claimTx.wait();
    logInfo(`ETH Claim Tx: https://sepolia.etherscan.io/tx/${claimTx.hash}`);
    logInfo("Secret has been revealed on Ethereum. Funds sent to maker.");
  } catch (e) {
    console.error("Failed to simulate maker's claim:", e);
    return;
  }

  // --- STEP 4: RESOLVER CLAIMS ON NEAR ---
  logStep(4, "Using the revealed secret to claim the wNEAR on the NEAR HTLC...");
  try {
    const nearClaimTx = await resolverNearAccount.functionCall({
      contractId: nearHtlcContract.accountId,
      methodName: "withdraw",
      args: {
        secret: secret_base64, // The NEAR contract expects base64
      },
      gas: "100000000000000" // 100 TGas
    });
    logInfo(`NEAR Claim Tx: https://testnet.nearblocks.io/txns/${nearClaimTx.transaction.hash}`);
    logInfo("Resolver has claimed the wNEAR.");
  } catch (e) {
    console.error("Failed to claim funds on NEAR:", e);
    return;
  }

  console.log("\n✅✅✅ Cross-chain swap successfully completed! ✅✅✅");
}

main().catch(console.error);

import type { WalletSelector } from "@near-wallet-selector/core";
import { Account } from "@near-js/accounts";
import { Provider } from "@near-js/providers";
import { sha256 } from "js-sha256";
import { toast } from "sonner";
import bs58 from "bs58";
import { parseNearAmount } from "@near-js/utils";
import { serialize, Schema } from "borsh"

const HTLC_CONTRACT_ID = "ccsn2.testnet";

export const syncPublicKeys = async (selector: WalletSelector, accountId: string, provider: Provider) => {
  const wallet = await selector.wallet();
  const acc = new Account(accountId, provider);
  const res = await acc.getAccessKeyList();
  const allKeys = new Set(res.keys.filter((v) => v.access_key.permission === "FullAccess").map((v) => v.public_key));
  console.log("AllKeys", allKeys)
  if (!allKeys || !allKeys.size) {
    return
  }
  console.log("AllKeys", allKeys)

  const registeredKeys = await getRegisteredKeys(accountId, provider)
  console.log("RegisteredKeys", registeredKeys)

  const keysToRegister = new Set([...allKeys].filter(k => !registeredKeys.has(k)));
  const keysToUnRegister = new Set([...registeredKeys].filter(k => !allKeys.has(k)));

  console.log("KeysToRegister", keysToRegister)
  console.log("KeysToUnRegister", keysToUnRegister)

  const promises = [];

  if (keysToRegister.size) {
    promises.push(wallet.signAndSendTransaction({
      signerId: accountId,
      receiverId: HTLC_CONTRACT_ID,
      actions: [
        {
          type: "FunctionCall",
          params: {
            methodName: "register_keys",
            args: {
              public_keys: Array.from(keysToRegister),
            },
            gas: "30000000000000",
            deposit: "0",
          },
        },
      ],
    }));
  }
  if (keysToUnRegister.size) {
    promises.push(wallet.signAndSendTransaction({
      signerId: accountId,
      receiverId: HTLC_CONTRACT_ID,
      actions: [
        {
          type: "FunctionCall",
          params: {
            methodName: "unregister_keys",
            args: {
              public_keys: Array.from(keysToUnRegister),
            },
            gas: "30000000000000",
            deposit: "0",
          },
        },
      ],
    }));
  }

  return await Promise.all(promises)
};

export const getRegisteredKeys = async (
  accountId: string,
  provider: Provider
) => {
  const registeredKeys: string[] = await provider.callFunction(
    HTLC_CONTRACT_ID,
    "get_registered_keys",
    {
      account_id: accountId,
    },
  );

  return new Set(registeredKeys);
};




// --- Deposit & Balance Management ---


const THIRTY_TGAS = "30000000000000";
const ONE_HUNDRED_TGAS = "100000000000000";

/**
 * Deposits a specified amount of a Fungible Token into the HTLC contract internal ledger.
 */
export const makeDeposit = async (
  selector: WalletSelector,
  accountId: string,
  tokenContractId: string,
  amount: string // In human readable format e.g. "1.5"
) => {
  const wallet = await selector.wallet();
  const depositAmount = parseNearAmount(amount); // Converts to yocto-units
  if (!depositAmount) throw new Error("Invalid deposit amount");

  return wallet.signAndSendTransaction({
    signerId: accountId,
    receiverId: tokenContractId,
    actions: [{
      type: "FunctionCall",
      params: {
        methodName: "ft_transfer_call",
        args: {
          receiver_id: HTLC_CONTRACT_ID,
          amount: depositAmount,
          msg: JSON.stringify({ type: "Deposit" }),
        },
        gas: ONE_HUNDRED_TGAS,
        deposit: "1",
      },
    }],
  });
};

/**
 * Queries the HTLC contract for the user's available (non-escrowed) balance for a specific token.
 */
export const getAvailableBalance = async (
  provider: Provider,
  accountId: string,
  tokenContractId: string
): Promise<string> => { // Returns balance in yocto-units
  try {
    return await provider.callFunction(
      HTLC_CONTRACT_ID,
      "get_available_balance",
      {
        account_id: accountId,
        token_id: tokenContractId
      },
    );
  } catch (error) {
    console.error("Error fetching available balance:", error);
    return "0";
  }
};


// --- NEW: Off-chain Intent Signing ---

class Timelocks {
  constructor(
    public src_withdrawal_delay: number,
    public src_public_withdrawal_delay: number,
    public src_cancellation_delay: number,
    public src_public_cancellation_delay: number,
    public dst_withdrawal_delay: number,
    public dst_public_withdrawal_delay: number,
    public dst_cancellation_delay: number,
  ) { }
}

class SignedOrder {
  constructor(
    public maker_id: string,
    public asset_id: string,
    public amount: bigint, // yocto
    public hashlock: number[],
    public timelocks: Timelocks,
    public nonce: number
  ) { }

  encode(): Uint8Array {
    return serialize(schema, this);
  }
}

// Define Timelocks type structure
const timelocksSchema = {
  struct: {
    src_withdrawal_delay: 'u64',
    src_public_withdrawal_delay: 'u64',
    src_cancellation_delay: 'u64',
    src_public_cancellation_delay: 'u64',
    dst_withdrawal_delay: 'u64',
    dst_public_withdrawal_delay: 'u64',
    dst_cancellation_delay: 'u64',
  },
};

// Define full schema
const schema = {
  struct: {
    maker_id: 'string',
    asset_id: 'string',
    amount: 'u128',
    hashlock: { array: { type: 'u8' } },
    timelocks: timelocksSchema,
    nonce: 'u128',
  },
};


/**
 * Creates and signs a Fusion+ swap order (intent).
 * This is an OFF-CHAIN action that produces a payload for a resolver.
 */
export const createAndSignSwapOrder = async (
  selector: WalletSelector,
  accountId: string,
  provider: Provider,
  params: {
    sourceTokenContract: string,
    sourceAmount: string, // human-readable amount
    // Other params for the order...
  }
) => {
  const wallet = await selector.wallet();
  if (!wallet.signMessage) {
    toast.error("Wallet does not support message signing. Please use a wallet like MyNearWallet or HERE Wallet.");
    throw new Error("signMessage is not supported");
  }

  // 1. Generate Secret and Hash
  const secret = window.crypto.getRandomValues(new Uint8Array(32));
  const hash = sha256.create().update(secret).array();
  const hashlock_b58 = bs58.encode(Buffer.from(hash));
  console.log("Generated Secret (Base64):", Buffer.from(secret).toString('base64'));
  console.log("Generated Hash (Base58):", hashlock_b58);

  // 2. Construct the Order Payload (must match Rust `SignedOrder` struct)
  const orderToSign = new SignedOrder(
    accountId,
    params.sourceTokenContract,
    BigInt(parseNearAmount(params.sourceAmount)!), // yocto
    Array.from(hash),
    { // Example Timelocks (in seconds). These should be agreed upon.
      src_withdrawal_delay: 3600, // 1 hr
      src_public_withdrawal_delay: 7200, // 2 hrs
      src_cancellation_delay: 14400, // 4 hrs
      src_public_cancellation_delay: 28800, // 8 hrs
      dst_withdrawal_delay: 1800, // 30 min (must be shorter than src cancel)
      dst_public_withdrawal_delay: 3600, // 1 hr
      dst_cancellation_delay: 10800, // 3 hr
    },
    Date.now() // Use timestamp as a simple nonce
  );

  const signatureNonce = window.crypto.getRandomValues(new Uint8Array(32));

  // 3. Sign the serialized payload
  const payload = orderToSign.encode();
  const signature = await wallet.signMessage({
    message: Buffer.from(payload).toString('base64'), // Sign the raw bytes as base64
    recipient: HTLC_CONTRACT_ID,
    nonce: Buffer.from(signatureNonce),
  });

  console.log("message nonce", signatureNonce)

  // 4. Assemble the final package for the resolver
  const resolverPayload = {
    params: {
      maker_id: orderToSign.maker_id,
      asset_id: orderToSign.asset_id,
      amount: orderToSign.amount,
      hashlock: hashlock_b58,
      timelocks: orderToSign.timelocks,
      nonce: orderToSign.nonce,
    },
    signature: signature!.signature,
    public_key: signature!.publicKey,
  };

  console.log("Resolver Payload:", resolverPayload);
  toast.success("Swap Intent Signed! See console for resolver payload.");

  // Also show the secret to the user so they can claim on ETH
  toast.info(`Your secret to claim on Ethereum is: ${Buffer.from(secret).toString('base64')}`);

  return resolverPayload;
};

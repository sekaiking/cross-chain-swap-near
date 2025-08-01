## NEAR Fusion+ HTLC Contract

### Overview

This smart contract implements a 1inch Fusion+-style cross-chain atomic swap protocol on the NEAR blockchain. It is designed to facilitate trustless, bidirectional swaps between NEAR (using NEP-141 tokens) and EVM chains like Ethereum.

The core challenge this contract solves is adapting the Fusion+ "intent-based" model, where a maker signs an off-chain order and a competitive resolver executes it, to NEAR's architecture which lacks the token `allowance` (pull) mechanism common in EVM. It achieves this by using a non-custodial **internal deposit ledger**.

### Core Features

* **Internal Deposit Ledger:** Makers deposit tokens into the contract once. This balance can then be used for multiple, gas-less (for the maker) swaps without requiring further on-chain action from them.
* **Off-chain Intent Signing:** Makers sign a cryptographic message (`SignedOrder`) declaring their intent to swap, preserving the gas-less, non-custodial experience of Fusion+.
* **Resolver-Driven Execution:** A competitive market of resolvers can pick up these signed intents. The first resolver to act on-chain locks the maker's deposited funds into an escrow on their behalf.
* **Atomic Swaps via HTLC:** Utilizes the classic Hashed Timelock Contract pattern. Swaps are atomic: either both parties receive their funds, or no one does after a timeout.
* **Bidirectional Support:** The contract handles both swap directions (NEAR -> Other, Other -> NEAR) through a unified `ft_on_transfer` entry point and dedicated logic.
* **Safety Mechanisms:** Employs nonces to prevent replay attacks, explicit timelocks for claims and refunds, and a resolver "safety deposit" to incentivize completion.

### Example Usage & Flows

**Assumptions:**

* Contract deployed at `htlc.near`.
* wNEAR token at `wnear.testnet`.
* Maker account: `maker.near`.
* Resolver account: `resolver.near`.
* **Off-chain Action:** Maker generates a secret (`S`) and its hash (`H`).
  * Secret (base64): `c2VjcmV0`
  * Hash (base58): `H4Fk7cQ1D5w4g3j2K1F9h8g7F5d4C3b2A1`

#### Flow 1: NEAR → Ethereum (Maker Sells wNEAR for ETH)

##### Setup Phase

**Maker deposits wNEAR into NEAR contract** - The maker calls `ft_transfer_call` on the wNEAR token contract to deposit tokens into the HTLC contract. This creates an internal balance that can be used for multiple swaps without additional on-chain actions.

**Maker generates secret and signs order** - The maker generates a secret `S` and computes its hash `H`. They then sign an off-chain Fusion+ order containing the hash `H`, specifying they want to swap wNEAR for ETH. This signed order is broadcast to resolvers.

##### Execution Phase

**Resolver locks maker's wNEAR** - A resolver calls `initiate_source_escrow` on the NEAR contract, providing the maker's signed order. This locks the maker's deposited wNEAR in an escrow that can only be unlocked by revealing secret `S`. The locked funds are payable to the resolver.

**Resolver locks their own ETH** - The resolver creates a corresponding escrow on Ethereum, locking their own ETH with the same hash `H`. This escrow is payable to the maker and has a shorter timelock than the NEAR escrow.

**Maker reveals secret to claim ETH** - The maker calls the Ethereum contract with secret `S` to claim the resolver's ETH. By revealing the secret on Ethereum, it becomes publicly visible on that chain.

**Resolver uses revealed secret** - The resolver monitors Ethereum, sees the revealed secret `S`, and uses it to call `withdraw` on the NEAR contract to claim the maker's locked wNEAR.

#### Flow 2: Ethereum → NEAR (Maker Buys wNEAR with ETH)

##### Setup Phase

**Maker signs authorization order** - The maker generates secret `S` and hash `H`, then signs an off-chain order authorizing a resolver to lock their ETH in an Ethereum escrow. No pre-deposit is required on NEAR.

##### Execution Phase

**Resolver locks maker's ETH** - The resolver calls the Ethereum HTLC contract, providing the maker's signed authorization. This locks the maker's ETH in an escrow with hash `H` that's payable to the resolver.

**Resolver locks their own wNEAR** - The resolver calls `ft_transfer_call` on the wNEAR token to create a destination escrow on NEAR. They send their own wNEAR to the HTLC contract with message parameters specifying the hash `H` and that the escrow is payable to the maker.

**Maker reveals secret to claim wNEAR** - The maker calls `withdraw` on the NEAR contract with secret `S` to claim the resolver's wNEAR. The secret becomes publicly visible on NEAR.

**Resolver uses revealed secret** - The resolver monitors NEAR, sees the revealed secret `S`, and uses it on Ethereum to claim the maker's ETH from the source escrow.

### Key Differences Between Flows

**Flow 1 (NEAR→Ethereum)**: Maker must pre-deposit on NEAR and reveals secret on Ethereum (destination chain) to claim their desired ETH.

**Flow 2 (Ethereum→NEAR)**: No pre-deposit required, resolver handles all on-chain setup, and maker reveals secret on NEAR (destination chain) to claim their desired wNEAR.

### Safety Mechanisms

**Timelock Protection**: Each escrow has timelocks. If the maker doesn't reveal the secret in time, both parties can reclaim their original assets. The destination chain (where maker claims) always has a shorter timelock than the source chain.

**Nonce Prevention**: Signed orders include nonces to prevent replay attacks where the same order could be executed multiple times.

**Resolver Incentives**: Resolvers provide safety deposits when creating escrows, creating economic incentives to complete swaps properly or execute refunds if needed.

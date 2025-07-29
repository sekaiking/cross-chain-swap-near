"use client";

import { SwapCard } from "@/components/SwapCard";
import { useAccount } from "wagmi";
import { ethTokens, nearTokens } from "@/lib/tokens";

export default function EthToNearPage() {
  const { isConnected: isEvmConnected } = useAccount();

  return (
    <main className="flex flex-1 flex-col items-center justify-center p-6">
      <SwapCard
        sourceChainName="Ethereum"
        sourceTokenList={ethTokens}
        destChainName="NEAR"
        destTokenList={nearTokens}
        destAddressPlaceholder="recipient.testnet"
        isSourceConnected={isEvmConnected}
      />
    </main>
  );
}

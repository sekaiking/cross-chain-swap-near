"use client";

import { useState } from "react";
import { SwapCard } from "@/components/SwapCard";
import { useAccount } from "wagmi";
import { ethTokens, nearTokens, Token } from "@/lib/tokens";
import { EthSetupChecklist } from "@/components/EthSetupChecklist"; // Import new component

export default function EthToNearPage() {
  const { isConnected: isEvmConnected } = useAccount();

  // Lift state for selected tokens up to the page level
  const [sourceToken, setSourceToken] = useState<Token>(ethTokens[0]);
  const [destToken, setDestToken] = useState<Token>(nearTokens[0]);

  return (
    <main className="flex flex-1 flex-col items-center justify-center p-6">
      {isEvmConnected && (
        <EthSetupChecklist selectedSourceToken={sourceToken} />
      )}

      <SwapCard
        sourceChainName="Ethereum"
        sourceTokenList={ethTokens}
        destChainName="NEAR"
        destTokenList={nearTokens}
        destAddressPlaceholder="recipient.testnet"
        isSourceConnected={isEvmConnected}
        // Pass state and setters down to SwapCard
        sourceToken={sourceToken}
        setSourceToken={setSourceToken}
        destToken={destToken}
        setDestToken={setDestToken}
      />
    </main>
  );
}

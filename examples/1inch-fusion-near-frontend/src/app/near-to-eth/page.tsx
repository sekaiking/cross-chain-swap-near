"use client";

import { SwapCard } from "@/components/SwapCard";
import { NearWalletContext } from "@/app/providers";
import { useContext } from "react";
import { ethTokens, nearTokens } from "@/lib/tokens";

const useNearWallet = () => {
  const context = useContext(NearWalletContext);
  if (!context) throw new Error("useNearWallet must be used within a NearWalletProvider");
  return context;
};

export default function NearToEthPage() {
  const { accountId: nearAccountId } = useNearWallet();

  return (
    <main className="flex flex-1 flex-col items-center justify-center p-6">
      <SwapCard
        sourceChainName="NEAR"
        sourceTokenList={nearTokens}
        destChainName="Ethereum"
        destTokenList={ethTokens}
        destAddressPlaceholder="0x...your-eth-address"
        isSourceConnected={!!nearAccountId}
      />
    </main>
  );
}

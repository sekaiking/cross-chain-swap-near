"use client";

import { SwapCard } from "@/components/SwapCard";
import { NearWalletContext } from "@/app/providers";
import { useContext, useEffect, useState } from "react";
import { ethTokens, nearTokens, Token } from "@/lib/tokens";
import { syncPublicKeys } from "@/lib/near-interactions";
import { toast } from "sonner";
import { DepositManager } from "@/components/NearDepositManager";

const useNearWallet = () => {
  const context = useContext(NearWalletContext);
  if (!context) throw new Error("useNearWallet must be used within a NearWalletProvider");
  return context;
};

export default function NearToEthPage() {
  const { selector, provider, accountId: nearAccountId } = useNearWallet();
  const [sourceToken, setSourceToken] = useState<Token>(nearTokens[0]);
  const [destToken, setDestToken] = useState<Token>(ethTokens[0]);

  // Sync public keys on login
  useEffect(() => {
    if (selector && provider && nearAccountId) {
      syncPublicKeys(selector, nearAccountId, provider)
        .then(() => console.log("Public keys synced successfully."))
        .catch(err => {
          console.error("Failed to sync public keys:", err)
          toast.error("Failed to sync public keys. Check console for details.")
        });
    }
  }, [selector, provider, nearAccountId]);

  return (
    <main className="flex flex-1 flex-col items-center justify-center p-6">
      {nearAccountId && (
        <DepositManager
          accountId={nearAccountId}
          selectedToken={sourceToken}
        />
      )}
      <SwapCard
        sourceChainName="NEAR"
        sourceTokenList={nearTokens}
        destChainName="Ethereum"
        destTokenList={ethTokens}
        destAddressPlaceholder="0x...your-eth-address"
        isSourceConnected={!!nearAccountId}
        sourceToken={sourceToken}
        setSourceToken={setSourceToken}
        destToken={destToken}
        setDestToken={setDestToken}
      />
    </main>
  );
}

"use client";

import * as React from "react";
import { RainbowKitProvider, getDefaultConfig } from "@rainbow-me/rainbowkit";
import { WagmiProvider } from "wagmi";
import { mainnet, sepolia } from "wagmi/chains";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { setupWalletSelector, WalletSelector } from "@near-wallet-selector/core";
import { setupModal } from "@near-wallet-selector/modal-ui";
import { setupMyNearWallet } from "@near-wallet-selector/my-near-wallet";
import { setupSender } from "@near-wallet-selector/sender";
import type { AccountState } from "@near-wallet-selector/core";

interface NearWalletContextType {
  selector: WalletSelector;
  modal: any;
  accounts: Array<AccountState>;
  accountId: string | null;
}

export const NearWalletContext = React.createContext<NearWalletContextType | null>(null);

const wagmiConfig = getDefaultConfig({
  appName: "Fusion++Near",
  projectId: "3f17b2190776d405bc701bcae1e98435",
  chains: [sepolia, mainnet],
  ssr: true,
});

const queryClient = new QueryClient();

// Main Provider Component
export function Providers({ children }: { children: React.ReactNode }) {
  const [selector, setSelector] = React.useState<WalletSelector | null>(null);
  const [modal, setModal] = React.useState<any>(null);
  const [accounts, setAccounts] = React.useState<Array<AccountState>>([]);

  React.useEffect(() => {
    const setupSelector = async () => {
      const _selector = await setupWalletSelector({
        network: "testnet",
        modules: [
          setupMyNearWallet(),
          setupSender(),
        ],
      });

      const _modal = setupModal(_selector, {
        contractId: "ccsn1.testnet",
      });
      const state = _selector.store.getState();
      setAccounts(state.accounts);

      setSelector(_selector);
      setModal(_modal);
    };

    setupSelector();
  }, []);

  const accountId = accounts.find((account) => account.active)?.accountId || null;

  const nearContextValue = React.useMemo<NearWalletContextType>(() => ({
    selector: selector!,
    modal: modal!,
    accounts,
    accountId,
  }), [selector, modal, accounts, accountId]);

  return (
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider>
          <NearWalletContext.Provider value={nearContextValue}>
            {children}
          </NearWalletContext.Provider>
        </RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
}

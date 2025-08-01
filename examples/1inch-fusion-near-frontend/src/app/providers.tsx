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
import { JsonRpcProvider, Provider } from "@near-js/providers";
import { syncPublicKeys } from "@/lib/near-interactions";
import { toast } from "sonner";

interface NearWalletContextType {
  selector: WalletSelector;
  modal: any;
  accounts: Array<AccountState>;
  accountId: string | null;
  provider: Provider;
  ready: boolean;
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
  const [nearProvider, setNearProvider] = React.useState<Provider>()
  const [syncingPublicKeys, setSyncingPublicKeys] = React.useState<boolean>(false);
  const [syncedPublicKeys, setSyncedPublicKeys] = React.useState<boolean>();

  React.useEffect(() => {
    const setupSelector = async () => {
      const _selector = await setupWalletSelector({
        network: "testnet",
        fallbackRpcUrls: ["https://test.rpc.fastnear.com"],
        modules: [
          setupMyNearWallet(),
          setupSender(),
        ],
      });

      const _modal = setupModal(_selector, {
        contractId: "ccsn2.testnet",
      });
      const state = _selector.store.getState();
      setAccounts(state.accounts);

      setSelector(_selector);
      setModal(_modal);
    };

    setNearProvider((new JsonRpcProvider({ url: "https://test.rpc.fastnear.com" })) as Provider);

    setupSelector();
  }, []);


  const accountId = accounts.find((account) => account.active)?.accountId || null;

  // Effect for syncing keys, runs only when accountId appears
  React.useEffect(() => {
    // Guard: only run if we have the necessary pieces and haven't synced yet.
    if (!selector || !accountId || !nearProvider || syncedPublicKeys) {
      return;
    }

    let isSubscribed = true;
    const sync = async () => {
      toast("Syncing NEAR Public Keys... Please wait, this is a one-time setup.");
      try {
        await syncPublicKeys(selector, accountId, nearProvider);
        if (isSubscribed) {
          toast.success("NEAR Ready: Public keys are synced with the contract.");
          setSyncedPublicKeys(true);
        }
      } catch (error) {
        console.error("Failed to sync public keys:", error);
        toast.error("Could not sync public keys. Please try to reconnect and try again.");
      }
    };

    sync();

    return () => { isSubscribed = false; };
  }, [selector, accountId, nearProvider, syncedPublicKeys, toast]);


  const nearContextValue = React.useMemo<NearWalletContextType>(() => ({
    selector: selector!,
    modal,
    accounts,
    accountId,
    provider: nearProvider!,
    ready: (!!selector && !!accountId && !!nearProvider && syncedPublicKeys) || false,
  }), [selector, modal, accounts, accountId, nearProvider, syncedPublicKeys]);

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

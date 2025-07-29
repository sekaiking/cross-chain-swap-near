"use client";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useContext } from "react";
import { NearWalletContext } from "@/app/providers";

const useNearWallet = () => {
  const context = useContext(NearWalletContext);
  if (!context) {
    throw new Error("useNearWallet must be used within a NearWalletProvider");
  }
  return context;
};

const NearConnectButton = () => {
  const { modal, accountId, selector } = useNearWallet();

  const handleConnect = () => modal?.show();

  const handleDisconnect = async () => {
    if (!selector) return;
    const wallet = await selector.wallet();
    await wallet.signOut();
    window.location.reload();
  };

  if (accountId) {
    return (
      <Button variant="outline" onClick={handleDisconnect} className="h-9">
        {accountId.length > 12
          ? `${accountId.substring(0, 6)}...${accountId.slice(-4)}`
          : accountId}
      </Button>
    );
  }

  return (
    <Button variant="outline" onClick={handleConnect} className="cursor-pointer h-9">
      Connect NEAR
    </Button>
  );
};

export function Navbar() {
  const pathname = usePathname();

  const navLinks = [
    { href: "/eth-to-near", label: "ETH → NEAR" },
    { href: "/near-to-eth", label: "NEAR → ETH" },
  ];

  const showNearButton = pathname === "/near-to-eth";
  const showEthButton = pathname === "/eth-to-near";

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto px-4">
        <div className="flex h-16 items-center justify-between">
          <Link href="/" className="flex items-center space-x-2">
            <span className="text-xl font-bold">Fusion++Near</span>
          </Link>

          <nav className="hidden md:flex items-center space-x-8">
            {navLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "px-3 py-2 rounded-md text-sm font-medium transition-colors",
                  pathname === link.href
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted"
                )}
              >
                {link.label}
              </Link>
            ))}
          </nav>

          <nav className="flex md:hidden items-center space-x-4">
            {navLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "px-2 py-1 rounded text-xs font-medium transition-colors",
                  pathname === link.href
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {link.label}
              </Link>
            ))}
          </nav>

          <div className="flex items-center space-x-3">
            {showNearButton && <NearConnectButton />}
            {showEthButton && (
              <ConnectButton
                chainStatus="icon"
                showBalance={false}
                accountStatus="avatar"
              />
            )}
          </div>
        </div>
      </div>
    </header>
  );
}

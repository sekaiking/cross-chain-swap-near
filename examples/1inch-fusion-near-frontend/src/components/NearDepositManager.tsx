"use client";

import { useEffect, useState, useContext } from "react";
import { NearWalletContext } from "@/app/providers";
import { Token } from "@/lib/tokens";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { getAvailableBalance, makeDeposit } from "@/lib/near-interactions";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";
import { formatNearAmount } from "@near-js/utils";

interface DepositManagerProps {
  accountId: string;
  selectedToken: Token;
}

export function DepositManager({ accountId, selectedToken }: DepositManagerProps) {
  const { selector, provider } = useContext(NearWalletContext)!;
  const [balance, setBalance] = useState("0");
  const [depositAmount, setDepositAmount] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isDepositing, setIsDepositing] = useState(false);

  useEffect(() => {
    const fetchBalance = async () => {
      if (!provider || !accountId || !selectedToken) return;
      setIsLoading(true);
      try {
        const bal = await getAvailableBalance(provider, accountId, selectedToken.address);
        setBalance(bal);
      } catch (e) {
        console.error("Failed to fetch balance:", e);
        setBalance("0");
      } finally {
        setIsLoading(false);
      }
    };
    fetchBalance();
  }, [provider, accountId, selectedToken]);

  const handleDeposit = async () => {
    if (!selector || !accountId || !depositAmount || parseFloat(depositAmount) <= 0) {
      toast.error("Please enter a valid amount to deposit.");
      return;
    }
    setIsDepositing(true);
    try {
      await makeDeposit(selector, accountId, selectedToken.address, depositAmount);
      toast.success("Deposit transaction sent! The balance will update shortly.");
      // Note: Balance won't update instantly. A full solution might use a subscription or polling.
    } catch (e) {
      console.error(e);
      toast.error("Failed to send deposit transaction.");
    } finally {
      setIsDepositing(false);
    }
  };

  const formattedBalance = formatNearAmount(balance, selectedToken.decimals);

  return (
    <Card className="w-[450px] mb-4">
      <CardHeader>
        <CardTitle>Step 1: Deposit Funds</CardTitle>
        <CardDescription>
          Deposit {selectedToken.symbol} into the swap contract to make them available for atomic swaps. This is a one-time setup per token.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="mb-4">
          <Label>Your Available Balance in Contract</Label>
          <div className="text-2xl font-bold">
            {isLoading ? <Loader2 className="h-6 w-6 animate-spin" /> : `${parseFloat(formattedBalance).toFixed(4)} ${selectedToken.symbol}`}
          </div>
        </div>
        <div className="flex gap-2">
          <Input
            placeholder={`Amount of ${selectedToken.symbol}`}
            value={depositAmount}
            onChange={(e) => setDepositAmount(e.target.value)}
            type="number"
            disabled={isDepositing}
          />
          <Button onClick={handleDeposit} disabled={isDepositing}>
            {isDepositing && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Deposit
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

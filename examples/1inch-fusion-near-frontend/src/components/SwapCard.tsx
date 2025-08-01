"use client";

import React, { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import { ArrowDown } from "lucide-react";
import { createAndSignSwapOrder } from "@/lib/near-interactions";
import { NearWalletContext } from "@/app/providers";
import { useContext } from "react";
import { Loader2 } from "lucide-react";

import { Token } from "@/lib/tokens";
import { TokenSelector } from "./TokenSelector";

interface SwapCardProps {
  sourceChainName: string;
  sourceTokenList: Token[];
  destChainName: string;
  destTokenList: Token[];
  destAddressPlaceholder: string;
  isSourceConnected: boolean;
  sourceToken: Token;
  setSourceToken: (token: Token) => void;
  destToken: Token;
  setDestToken: (token: Token) => void;
}

export function SwapCard({
  sourceChainName,
  sourceTokenList,
  destChainName,
  destTokenList,
  destAddressPlaceholder,
  isSourceConnected,
  sourceToken,
  setSourceToken,
  destToken,
  setDestToken,
}: SwapCardProps) {
  const { selector, provider, accountId } = useContext(NearWalletContext)!;
  const [isSigning, setIsSigning] = useState(false);
  const [sourceAmount, setSourceAmount] = useState("");
  const [receiveAmount, setReceiveAmount] = useState("0.0");


  // Simulate a fixed exchange rate for display
  useEffect(() => {
    const MOCK_NEAR_TO_ETH_RATE = 0.0016;
    const amount = parseFloat(sourceAmount);
    if (!isNaN(amount) && amount > 0) {
      setReceiveAmount((amount * MOCK_NEAR_TO_ETH_RATE).toFixed(5));
    } else {
      setReceiveAmount("0.0");
    }
  }, [sourceAmount, sourceToken, destToken]);

  const handleSwap = async () => {
    if (!selector || !provider || !accountId || !sourceAmount || parseFloat(sourceAmount) <= 0) {
      toast.error("Please connect your wallet and enter a valid amount.");
      return;
    }

    setIsSigning(true);
    try {
      await createAndSignSwapOrder(selector, accountId, provider, {
        sourceTokenContract: sourceToken.address,
        sourceAmount: sourceAmount,
      });
      // The function internally shows toasts on success/failure
    } catch (err) {
      console.error("Signing failed:", err);
      // The interaction lib already shows a toast on most errors
    } finally {
      setIsSigning(false);
    }
  };

  return (
    <Card className="w-[450px]">
      <CardHeader>
        <CardTitle>Step 2: Create Swap Intent</CardTitle>
        <CardDescription>
          Sign an off-chain message to authorize the swap. This is gas-less.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid w-full items-center gap-4">
          <div className="flex flex-col space-y-1.5">
            <Label htmlFor="send-amount">You Send</Label>
            <div className="flex gap-2">
              <Input
                id="send-amount"
                placeholder="0.0"
                className="flex-1"
                disabled={!isSourceConnected || isSigning}
                value={sourceAmount}
                onChange={(e) => setSourceAmount(e.target.value)}
                type="number"
              />
              <TokenSelector
                tokenList={sourceTokenList}
                selectedToken={sourceToken}
                setSelectedToken={setSourceToken}
                disabled={!isSourceConnected}
              />
            </div>
          </div>

          <div className="mx-auto my-[-10px] h-8 w-8 rounded-full border flex items-center justify-center bg-secondary">
            <ArrowDown className="h-4 w-4 text-muted-foreground" />
          </div>

          <div className="flex flex-col space-y-1.5">
            <Label htmlFor="receive-amount">You Receive</Label>
            <div className="flex gap-2">
              <Input
                id="receive-amount"
                placeholder="0.0"
                className="flex-1"
                value={receiveAmount}
                readOnly
              />
              <TokenSelector
                tokenList={destTokenList}
                selectedToken={destToken}
                setSelectedToken={setDestToken}
                disabled={!isSourceConnected}
              />
            </div>
          </div>

          <div className="flex flex-col space-y-1.5">
            <Label htmlFor="recipient">Recipient Address ({destChainName})</Label>
            <Input id="recipient" placeholder={destAddressPlaceholder} disabled={!isSourceConnected} />
          </div>
          <Button className="w-full mt-2" type="button" onClick={handleSwap} disabled={!isSourceConnected || isSigning}>
            {isSigning ? (
              <><Loader2 className="mr-2 h-4 w-4 animate-spin" /> Signing...</>
            ) : isSourceConnected ? (
              "Sign Swap Intent"
            ) : (
              `Connect ${sourceChainName} Wallet`
            )}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

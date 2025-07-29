"use client";

import React, { useState } from "react";
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

import { Token } from "@/lib/tokens";
import { TokenSelector } from "./TokenSelector";

interface SwapCardProps {
  sourceChainName: string;
  sourceTokenList: Token[];
  destChainName: string;
  destTokenList: Token[];
  destAddressPlaceholder: string;
  isSourceConnected: boolean;
}

export function SwapCard({
  sourceChainName,
  sourceTokenList,
  destChainName,
  destTokenList,
  destAddressPlaceholder,
  isSourceConnected,
}: SwapCardProps) {
  const [sourceToken, setSourceToken] = useState<Token>(sourceTokenList[0]);
  const [destToken, setDestToken] = useState<Token>(destTokenList[0]);

  const handleSwap = () => {
    console.log(`Initiating swap from ${sourceToken.symbol} to ${destToken.symbol}`);
    toast(JSON.stringify({
      title: "Swap Initiated (Simulation)",
      description: `Resolver logic for ${sourceChainName} -> ${destChainName} would run here.`,
    }, null, 2));
  };

  return (
    <Card className="w-[450px]">
      <CardHeader>
        <CardTitle>
          {sourceChainName} → {destChainName}
        </CardTitle>
        <CardDescription>
          Select tokens to swap atomically
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid w-full items-center gap-4">
          <div className="flex flex-col space-y-1.5">
            <Label htmlFor="send-amount">You Send</Label>
            <div className="flex gap-2">
              <Input id="send-amount" placeholder="0.0" className="flex-1" disabled={!isSourceConnected} />
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
              <Input id="receive-amount" placeholder="0.0" className="flex-1" disabled />
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
          <Button className="w-full mt-2" type="button" onClick={handleSwap} disabled={!isSourceConnected}>
            {isSourceConnected ? "Initiate Swap" : `Connect ${sourceChainName} Wallet`}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

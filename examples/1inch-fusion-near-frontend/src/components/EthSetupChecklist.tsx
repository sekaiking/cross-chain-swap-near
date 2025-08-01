"use client";

import { useEffect } from "react";
import { useWriteContract, useWaitForTransactionReceipt } from "wagmi";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Token } from "@/lib/tokens";
import { ERC20_ABI, ETH_ESCROW_CONTRACT_ADDRESS, MAX_APPROVAL_AMOUNT } from "@/lib/eth-constants";

interface EthSetupChecklistProps {
  selectedSourceToken: Token;
}

export function EthSetupChecklist({ selectedSourceToken }: EthSetupChecklistProps) {
  const { data: hash, error, isPending, writeContract } = useWriteContract();

  const handleApproveToken = () => {
    writeContract({
      address: selectedSourceToken.address as `0x${string}`,
      abi: ERC20_ABI,
      functionName: 'approve',
      args: [ETH_ESCROW_CONTRACT_ADDRESS, MAX_APPROVAL_AMOUNT],
    });
  };

  const { isLoading: isConfirming, isSuccess: isConfirmed } = useWaitForTransactionReceipt({
    hash,
  });

  useEffect(() => {
    if (isConfirming) {
      toast.loading("Waiting for transaction confirmation...");
    }
    if (isConfirmed) {
      toast.success(`${selectedSourceToken.symbol} approved for swapping.`);
    }
    if (error) {
      toast.error((error as any).shortMessage || error.message);
    }
  }, [isConfirming, isConfirmed, error, toast, selectedSourceToken.symbol]);


  return (
    <Card className="w-[450px] mb-6 bg-secondary">
      <CardHeader>
        <CardTitle className="text-lg">One-Time On-Chain Setup (per token)</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-between">
          <p className="text-sm font-medium">1. Approve {selectedSourceToken.symbol} for swaps</p>
          <Button
            variant="outline"
            size="sm"
            onClick={handleApproveToken}
            disabled={isPending || isConfirming}
          >
            {isPending || isConfirming ? 'Approving...' : 'Approve'}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

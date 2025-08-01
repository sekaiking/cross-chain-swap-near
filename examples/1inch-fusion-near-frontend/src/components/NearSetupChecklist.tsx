"use client";

import { useState, useEffect, useContext } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Token } from "@/lib/tokens";
import { NearWalletContext } from "@/app/providers";
import { approveToken } from "@/lib/near-interactions";
import { checkTokenApprovalStatus, setTokenApprovalStatus } from "@/lib/approval-manager";

interface NearSetupChecklistProps {
  accountId: string;
  selectedSourceToken: Token;
}

const useNearWallet = () => {
  const context = useContext(NearWalletContext);
  if (!context) throw new Error("useNearWallet must be used within a NearWalletProvider");
  return context;
};

export function NearSetupChecklist({ accountId, selectedSourceToken }: NearSetupChecklistProps) {
  const { selector, provider } = useNearWallet();

  const [isLoading, setIsLoading] = useState(true);
  const [isTokenApproved, setIsTokenApproved] = useState(false);

  // Effect to check status when component loads or inputs change
  useEffect(() => {
    if (!provider || !accountId) return;

    const checkStatus = async () => {
      setIsLoading(true);
      const tokenStatus = checkTokenApprovalStatus(accountId, selectedSourceToken.address);

      setIsTokenApproved(tokenStatus);
      setIsLoading(false);
    };

    checkStatus();
  }, [provider, accountId, selectedSourceToken]);


  const handleApproveToken = async () => {
    if (!selector) return;
    toast.loading("Approving fungible token on Near...")
    try {
      await approveToken(selector, accountId, selectedSourceToken.address);
      toast.success(`${selectedSourceToken.symbol} approved.`);
      setTokenApprovalStatus(accountId, selectedSourceToken.address);
      setIsTokenApproved(true);
    } catch (error) {
      toast.error((error as any).message || "Couldn't approve token")
    }
  };

  // If loading, show a placeholder
  if (isLoading) {
    return (
      <Card className="w-[450px] mb-6 bg-secondary/50 animate-pulse">
        <CardHeader><CardTitle className="text-lg">Checking Setup Status...</CardTitle></CardHeader>
        <CardContent><div className="h-10"></div></CardContent>
      </Card>
    );
  }

  if (isTokenApproved) {
    return null;
  }

  // Render the checklist with only the incomplete items
  return (
    <Card className="w-[450px] mb-6 bg-secondary">
      <CardHeader>
        <CardTitle className="text-lg">One-Time Setup</CardTitle>
      </CardHeader>

      <CardContent className="space-y-4">
        {!isTokenApproved && (
          <div className="flex items-center justify-between">
            <p className="text-sm font-medium">2. Approve {selectedSourceToken.symbol}</p>
            <Button variant="outline" size="sm" onClick={handleApproveToken}>Approve</Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

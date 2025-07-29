"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Token } from "@/lib/tokens";
import { ChevronDown } from "lucide-react";

interface TokenSelectorProps {
  tokenList: Token[];
  selectedToken: Token;
  setSelectedToken: (token: Token) => void;
  disabled?: boolean;
}

export function TokenSelector({
  tokenList,
  selectedToken,
  setSelectedToken,
  disabled = false,
}: TokenSelectorProps) {
  const [open, setOpen] = React.useState(false);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" className="w-[150px] justify-between" disabled={disabled}>
          <div className="flex items-center gap-2">
            <Avatar className="h-6 w-6">
              <AvatarImage src={selectedToken.logoURI} alt={selectedToken.name} />
              <AvatarFallback>{selectedToken.symbol.slice(0, 2)}</AvatarFallback>
            </Avatar>
            <span>{selectedToken.symbol}</span>
          </div>
          <ChevronDown className="h-4 w-4" />
        </Button>
      </DialogTrigger>
      <DialogContent className="p-0">
        <DialogHeader className="p-4 pb-0">
          <DialogTitle>Select a token</DialogTitle>
        </DialogHeader>
        <Command>
          <CommandInput placeholder="Search name or paste address..." />
          <CommandList>
            <CommandEmpty>No token found.</CommandEmpty>
            <ScrollArea className="h-[300px]">
              <CommandGroup>
                {tokenList.map((token) => (
                  <CommandItem
                    key={token.address}
                    value={`${token.symbol}-${token.address}`}
                    onSelect={() => {
                      setSelectedToken(token);
                      setOpen(false);
                    }}
                    className="flex items-center gap-3"
                  >
                    <Avatar className="h-8 w-8">
                      <AvatarImage src={token.logoURI} alt={token.name} />
                      <AvatarFallback>{token.symbol.slice(0, 2)}</AvatarFallback>
                    </Avatar>
                    <div>
                      <div className="font-medium">{token.symbol}</div>
                      <div className="text-xs text-muted-foreground">{token.name}</div>
                    </div>
                  </CommandItem>
                ))}
              </CommandGroup>
            </ScrollArea>
          </CommandList>
        </Command>
      </DialogContent>
    </Dialog>
  );
}

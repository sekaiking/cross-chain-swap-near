// TODO: the address deployed 1inch Escrow contract 
export const ETH_ESCROW_CONTRACT_ADDRESS = '0xYourEscrowContractAddressHere';

export const ERC20_ABI = [
  {
    "constant": false,
    "inputs": [
      { "name": "_spender", "type": "address" },
      { "name": "_value", "type": "uint256" }
    ],
    "name": "approve",
    "outputs": [{ "name": "", "type": "bool" }],
    "payable": false,
    "stateMutability": "nonpayable",
    "type": "function"
  }
] as const;

export const MAX_APPROVAL_AMOUNT = 2n ** 256n - 1n;

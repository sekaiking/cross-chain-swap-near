export interface Token {
  chainId: number | string;
  address: string;
  name: string;
  symbol: string;
  decimals: number;
  logoURI: string;
}

export const ethTokens: Token[] = [
  {
    chainId: 11155111, // Sepolia
    address: '0x7b79995e5f793A0722fcE19163Ee95459A897626', // WETH on Sepolia
    name: 'Wrapped Ether',
    symbol: 'WETH',
    decimals: 18,
    logoURI: 'https://assets.coingecko.com/coins/images/279/small/ethereum.png?1696501628'
  },
  {
    chainId: 11155111, // Sepolia
    address: '0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238', // USDC on Sepolia
    name: 'USD Coin',
    symbol: 'USDC',
    decimals: 6,
    logoURI: 'https://assets.coingecko.com/coins/images/6319/small/usdc.png?1696506694'
  },
];

export const nearTokens: Token[] = [
  {
    chainId: 'near:testnet',
    address: 'wrap.testnet',
    name: 'Wrapped NEAR',
    symbol: 'wNEAR',
    decimals: 24,
    logoURI: 'https://assets.coingecko.com/coins/images/10365/small/near.png?1696510367'
  },
  {
    chainId: 'near:testnet',
    address: 'usdc.fakes.testnet',
    name: 'USD Coin',
    symbol: 'USDC',
    decimals: 6,
    logoURI: 'https://assets.coingecko.com/coins/images/6319/small/usdc.png?1696506694'
  },
];

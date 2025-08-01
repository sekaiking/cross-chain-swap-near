import { Wallet } from 'ethers'
import { ethers } from "ethers";
import { randomBytes } from 'crypto';
import {
  SDK,
  HashLock,
  PrivateKeyProviderConnector,
  NetworkEnum,
} from "@1inch/cross-chain-sdk";



function getRandomBytes32(): string {
  return '0x' + randomBytes(32).toString('hex')
}

const privKey =
  '...'
const authKey = '...'
const nodeUrl = "https://sepolia.infura.io/v3/..."
const maker = new Wallet(privKey)

const makerPrivateKey = "...";
const makerAddress = "...";


const blockchainProvider = new PrivateKeyProviderConnector(
  makerPrivateKey,
  new ethers.JsonRpcProvider(process.env.ETH_RPC_URL)
);

const sdk = new SDK({
  url: "https://api.1inch.dev/fusion-plus",
  authKey: authKey,
  blockchainProvider,
});

const params = {
  srcChainId: NetworkEnum.ETHEREUM,
  dstChainId: 101010,
  srcTokenAddress: "...",
  dstTokenAddress: "...",
  amount: "1000000000000000000000",
  enableEstimate: true,
  walletAddress: makerAddress,
};

const quote = await sdk.getQuote(params);

const secretsCount = quote.getPreset().secretsCount;

const secrets = Array.from({ length: secretsCount }).map(() =>
  getRandomBytes32(),
);
const secretHashes = secrets.map((x) => HashLock.hashSecret(x));

const hashLock =
  secretsCount === 1
    ? HashLock.forSingleFill(secrets[0])
    : HashLock.forMultipleFills(
      secretHashes.map((secretHash, i) =>
        solidityPackedKeccak256(
          ["uint64", "bytes32"],
          [i, secretHash.toString()],
        ),
      ),
    );

sdk
  .createOrder(quote, {
    walletAddress: makerAddress,
    hashLock,
    secretHashes,
    // fee is an optional field
    fee: {
      takingFeeBps: 100, // 1% as we use bps format, 1% is equal to 100bps
      takingFeeReceiver: "0x0000000000000000000000000000000000000000", //  fee receiver address
    },
  })
  .then(console.log);

import { LimitOrder, MakerTraits, Address, Sdk, randBigInt, FetchProviderConnector } from "@1inch/limit-order-sdk"
import { Wallet } from 'ethers'

const privKey =
  '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
const authKey = '...'
const maker = new Wallet(privKey)
const expiresIn = 120n // 2m
const expiration = BigInt(Math.floor(Date.now() / 1000)) + expiresIn

const UINT_40_MAX = (1n << 48n) - 1n

// see MakerTraits.ts
const makerTraits = MakerTraits.default()
  .withExpiration(expiration)
  .withNonce(randBigInt(UINT_40_MAX))

const sdk = new Sdk({ authKey, networkId: 1, httpConnector: new FetchProviderConnector() })

const order = await sdk.createOrder({
  makerAsset: new Address('0xdac17f958d2ee523a2206206994597c13d831ec7'),
  takerAsset: new Address('0x111111111117dc0aa78b770fa6a738034120c302'),
  makingAmount: 100_000000n, // 100 USDT
  takingAmount: 10_00000000000000000n, // 10 1INCH
  maker: new Address(maker.address),
  // salt? : bigint
  // receiver? : Address
}, makerTraits)

const typedData = order.getTypedData()
const signature = await maker.signTypedData(
  typedData.domain,
  { Order: typedData.types.Order },
  typedData.message
)

await sdk.submitOrder(order, signature)

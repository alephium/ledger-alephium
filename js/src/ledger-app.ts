import { Account, KeyType, addressFromPublicKey, encodeHexSignature, groupOfAddress } from '@alephium/web3'
import Transport, { StatusCodes } from '@ledgerhq/hw-transport'
import * as serde from './serde'
import { ec as EC } from 'elliptic'
import { TokenMetadata } from './types'

const ec = new EC('secp256k1')

export const CLA = 0x80
export enum INS {
  GET_VERSION = 0x00,
  GET_PUBLIC_KEY = 0x01,
  SIGN_HASH = 0x02,
  SIGN_TX = 0x03
}

export const GROUP_NUM = 4
export const HASH_LEN = 32

// The maximum payload size is 255: https://github.com/LedgerHQ/ledger-live/blob/develop/libs/ledgerjs/packages/hw-transport/src/Transport.ts#L261
const MAX_PAYLOAD_SIZE = 255

export class AlephiumApp {
  readonly transport: Transport

  constructor(transport: Transport) {
    this.transport = transport
  }

  async close(): Promise<void> {
    await this.transport.close()
  }

  async getVersion(): Promise<string> {
    const response = await this.transport.send(CLA, INS.GET_VERSION, 0x00, 0x00)
    console.log(`response ${response.length} - ${response.toString('hex')}`)
    return `${response[0]}.${response[1]}.${response[2]}`
  }

  async getAccount(startPath: string, targetGroup?: number, keyType?: KeyType, display = false): Promise<readonly [Account, number]> {
    if ((targetGroup ?? 0) >= GROUP_NUM) {
      throw Error(`Invalid targetGroup: ${targetGroup}`)
    }

    if (keyType === 'bip340-schnorr') {
      throw Error('BIP340-Schnorr is not supported yet')
    }

    const p1 = targetGroup === undefined ? 0x00 : GROUP_NUM
    const p2 = targetGroup === undefined ? 0x00 : targetGroup
    const payload = Buffer.concat([serde.serializePath(startPath), Buffer.from([display ? 1 : 0])]);
    const response = await this.transport.send(CLA, INS.GET_PUBLIC_KEY, p1, p2, payload)
    const publicKey = ec.keyFromPublic(response.slice(0, 65)).getPublic(true, 'hex')
    const address = addressFromPublicKey(publicKey)
    const group = groupOfAddress(address)
    const hdIndex = response.slice(65, 69).readUInt32BE(0)

    return [{ publicKey: publicKey, address: address, group: group, keyType: keyType ?? 'default' }, hdIndex] as const
  }

  async signHash(path: string, hash: Buffer): Promise<string> {
    if (hash.length !== HASH_LEN) {
      throw new Error('Invalid hash length')
    }

    const data = Buffer.concat([serde.serializePath(path), hash])
    console.log(`data ${data.length}`)
    const response = await this.transport.send(CLA, INS.SIGN_HASH, 0x00, 0x00, data, [StatusCodes.OK])
    console.log(`response ${response.length} - ${response.toString('hex')}`)

    return decodeSignature(response)
  }

  async signUnsignedTx(
    path: string,
    unsignedTx: Buffer,
    tokenMetadata: TokenMetadata[] = []
  ): Promise<string> {
    console.log(`unsigned tx size: ${unsignedTx.length}`)
    const encodedPath = serde.serializePath(path)
    const encodedTokenMetadata = serde.serializeTokenMetadata(tokenMetadata)
    const firstFrameTxLength = MAX_PAYLOAD_SIZE - 20 - encodedTokenMetadata.length;
    const txData = unsignedTx.slice(0, unsignedTx.length > firstFrameTxLength ? firstFrameTxLength : unsignedTx.length)
    const data = Buffer.concat([encodedPath, encodedTokenMetadata, txData])
    let response = await this.transport.send(CLA, INS.SIGN_TX, 0x00, 0x00, data, [StatusCodes.OK])
    if (unsignedTx.length <= firstFrameTxLength) {
      return decodeSignature(response)
    }

    const frameLength = MAX_PAYLOAD_SIZE
    let fromIndex = firstFrameTxLength
    while (fromIndex < unsignedTx.length) {
      const remain = unsignedTx.length - fromIndex
      const toIndex = remain > frameLength ? (fromIndex + frameLength) : unsignedTx.length
      const data = unsignedTx.slice(fromIndex, toIndex)
      response = await this.transport.send(CLA, INS.SIGN_TX, 0x01, 0x00, data, [StatusCodes.OK])
      fromIndex = toIndex
    }

    return decodeSignature(response)
  }
}

function decodeSignature(response: Buffer): string {
  // Decode signature: https://bitcoin.stackexchange.com/a/12556
  const rLen = response.slice(3, 4)[0]
  const r = response.slice(4, 4 + rLen)
  const sLen = response.slice(5 + rLen, 6 + rLen)[0]
  const s = response.slice(6 + rLen, 6 + rLen + sLen)
  console.log(`${rLen} - ${r.toString('hex')}\n${sLen} - ${s.toString('hex')}`)
  return encodeHexSignature(r.toString('hex'), s.toString('hex'))
}
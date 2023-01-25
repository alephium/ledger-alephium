import { Account, addressFromPublicKey, encodeHexSignature, groupOfAddress } from '@alephium/web3'
import Transport, { StatusCodes } from '@ledgerhq/hw-transport'
import * as serde from './serde'
import { ec as EC } from 'elliptic'
import { assert } from 'console'

const ec = new EC('secp256k1')

export const CLA = 0x80
export enum INS {
  GET_VERSION = 0x00,
  GET_PUBLIC_KEY = 0x01,
  SIGN_HASH = 0x02
}

const GROUP_NUM = 4
const HASH_LEN = 32

export default class AlephiumApp {
  readonly transport: Transport

  constructor(transport: Transport) {
    this.transport = transport
  }

  async getVersion(): Promise<string> {
    const response = await this.transport.send(CLA, INS.GET_VERSION, 0x00, 0x00)
    console.log(`response ${response.length} - ${response.toString('hex')}`)
    return `${response[0]}.${response[1]}.${response[2]}`
  }

  // TODO: make address display optional
  async getAccount(startPath: string, targetGroup?: number): Promise<Account> {
    assert((targetGroup ?? 0) < GROUP_NUM)
    const p1 = targetGroup === undefined ? 0x00 : GROUP_NUM
    const p2 = targetGroup === undefined ? 0x00 : targetGroup
    const response = await this.transport.send(CLA, INS.GET_PUBLIC_KEY, p1, p2, serde.serializePath(startPath))
    console.log(`response ${response.length} - ${response.toString('hex')}`)
    const publicKey = ec.keyFromPublic(response.slice(0, 65)).getPublic(true, 'hex')
    console.log(`pubkey\n - ${publicKey}\n - ${response.toString('hex')}`)
    const address = addressFromPublicKey(publicKey)
    const group = groupOfAddress(address)

    return { publicKey: publicKey, address: address, group: group }
  }

  async signHash(path: string, hash: Buffer): Promise<string> {
    if (hash.length !== HASH_LEN) {
      throw new Error('Invalid hash length')
    }

    const data = Buffer.concat([serde.serializePath(path), hash])
    console.log(`data ${data.length}`)
    const response = await this.transport.send(CLA, INS.SIGN_HASH, 0x00, 0x00, data, [StatusCodes.OK])
    console.log(`response ${response.length} - ${response.toString('hex')}`)

    // Decode signature: https://bitcoin.stackexchange.com/a/12556
    const rLen = response.slice(3, 4)[0]
    const r = response.slice(4, 4 + rLen)
    const sLen = response.slice(5 + rLen, 6 + rLen)[0]
    const s = response.slice(6 + rLen, 6 + rLen + sLen)
    console.log(`${rLen} - ${r.toString('hex')}\n${sLen} - ${s.toString('hex')}`)
    return encodeHexSignature(r.toString('hex'), s.toString('hex'))
  }
}

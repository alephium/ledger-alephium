import { Account, addressFromPublicKey, encodeHexSignature, groupOfAddress } from '@alephium/web3'
import Transport, { StatusCodes } from '@ledgerhq/hw-transport'
import * as serde from './serde'
import { ec as EC } from 'elliptic'

const ec = new EC('secp256k1')

export const CLA = 0x80
export enum INS {
  GET_PUBLIC_KEY = 0x00,
  SIGN_HASH = 0x01
}

const HASH_LEN = 32

export default class AlephiumApp {
  readonly transport: Transport

  constructor(transport: Transport) {
    this.transport = transport
  }

  // TODO: make address display optional
  async getAccount(path: string): Promise<Account> {
    const response = await this.transport.send(CLA, INS.GET_PUBLIC_KEY, 0x00, 0x00, serde.serializePath(path))
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

    const response = await this.transport.send(
      CLA,
      INS.SIGN_HASH,
      0x00,
      0x00,
      Buffer.concat([serde.serializePath(path), hash]),
      [StatusCodes.OK]
    )
    const r = response.slice(0, 32)
    const s = response.slice(32, 64)
    return encodeHexSignature(r.toString('hex'), s.toString('hex'))
  }
}

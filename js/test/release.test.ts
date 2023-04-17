import NodeTransport from '@ledgerhq/hw-transport-node-hid'
import { listen } from '@ledgerhq/logs'
import blake from 'blakejs'

import { transactionVerifySignature } from '@alephium/web3'

import AlephiumApp from '../src'

describe.skip('Integration', () => {
  const path = `m/44'/1234'/0'/0/0`

  // enable this for integration test
  it('should test node', async () => {
    const transport = await NodeTransport.open('')
    listen((log) => console.log(log))
    const app = new AlephiumApp(transport)

    const account = await app.getAccount(path)
    console.log(`${JSON.stringify(account)}`)

    const hash = Buffer.from(blake.blake2b(Buffer.from([0, 1, 2, 3, 4]), undefined, 32))
    const signature = await app.signHash(path, hash)
    console.log(signature)
    expect(transactionVerifySignature(hash.toString('hex'), account.publicKey, signature)).toBe(true)

    await transport.close()
  }, 100000)
})

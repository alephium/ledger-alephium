import NodeTransport from '@ledgerhq/hw-transport-node-hid'
import { listen } from '@ledgerhq/logs'
import blake from 'blakejs'

import { transactionVerifySignature } from '@alephium/web3'

import AlephiumApp from '../src'

describe.skip('Integration', () => {
  const path = `m/44'/1234'/0'/0/0`

  // enable this for integration test
  it('should sign unsigned tx', async () => {
  })
})

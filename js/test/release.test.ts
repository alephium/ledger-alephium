import NodeTransport from '@ledgerhq/hw-transport-node-hid'
import { listen } from '@ledgerhq/logs'
import blake from 'blakejs'

import { ALPH_TOKEN_ID, Address, NodeProvider, ONE_ALPH, transactionVerifySignature } from '@alephium/web3'

import AlephiumApp from '../src'
import { getSigner, transfer } from '@alephium/web3-test'
import { waitTxConfirmed } from '@alephium/cli'

describe.skip('Integration', () => {
  const path = `m/44'/1234'/0'/0/0`
  const nodeProvider = new NodeProvider("http://127.0.0.1:22973")

  async function getALPHBalance(address: Address) {
    const balances = await nodeProvider.addresses.getAddressesAddressBalance(address)
    return BigInt(balances.balance)
  }

  async function transferToTestAccount(address: Address) {
    const fromAccount = await getSigner()
    const transferResult = await transfer(fromAccount, address, ALPH_TOKEN_ID, ONE_ALPH * 10n)
    await waitTxConfirmed(nodeProvider, transferResult.txId, 1, 1000)
    const balance0 = await getALPHBalance(address)
    expect(balance0).toEqual(ONE_ALPH * 10n)
  }

  // enable this for integration test
  it('should sign unsigned tx', async () => {
    const transport = await NodeTransport.open('')
    listen((log) => console.log(log))
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToTestAccount(testAccount.address)

    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7',
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
        },
        {
          address: '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7',
          attoAlphAmount: (ONE_ALPH * 3n).toString(),
        },
      ]
    })

    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    expect(transactionVerifySignature(buildTxResult.txId, testAccount.publicKey, signature)).toBe(true)

    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitTxConfirmed(nodeProvider, submitResult.txId, 1, 1000)
    const balance1 = await getALPHBalance(testAccount.address)
    expect(balance1 < (ONE_ALPH * 5n)).toEqual(true)

    await app.close()
  })
})

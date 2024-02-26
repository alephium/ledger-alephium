import NodeTransport from '@ledgerhq/hw-transport-node-hid'
import { listen } from '@ledgerhq/logs'

import { ALPH_TOKEN_ID, Address, NodeProvider, ONE_ALPH, transactionVerifySignature, web3 } from '@alephium/web3'

import AlephiumApp from '../src'
import { getSigner, mintToken, transfer } from '@alephium/web3-test'
import { waitTxConfirmed } from '@alephium/cli'
import { PrivateKeyWallet } from '@alephium/web3-wallet'

describe.skip('Integration', () => {
  const path = `m/44'/1234'/0'/0/0`
  const nodeProvider = new NodeProvider("http://127.0.0.1:22973")

  function randomP2PKHAddress(groupIndex: number): string {
    return PrivateKeyWallet.Random(groupIndex, nodeProvider).address
  }

  async function getALPHBalance(address: Address) {
    const balances = await nodeProvider.addresses.getAddressesAddressBalance(address)
    return BigInt(balances.balance)
  }

  async function transferToTestAccount(address: Address) {
    const fromAccount = await getSigner()
    const transferResult = await transfer(fromAccount, address, ALPH_TOKEN_ID, ONE_ALPH * 10n)
    await waitTxConfirmed(nodeProvider, transferResult.txId, 1, 1000)
  }

  // enable this for integration test
  it('should transfer ALPH', async () => {
    const transport = await NodeTransport.open('')
    listen((log) => console.log(log))
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToTestAccount(testAccount.address)

    const balance0 = await getALPHBalance(testAccount.address)
    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: randomP2PKHAddress(0),
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
        },
        {
          address: randomP2PKHAddress(0),
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
    const gasFee = BigInt(buildTxResult.gasAmount) * BigInt(buildTxResult.gasPrice)
    expect(balance1).toEqual(balance0 - gasFee - ONE_ALPH * 5n)

    await app.close()
  }, 120000)

  it('should transfer token', async () => {
    const transport = await NodeTransport.open('')
    listen((log) => console.log(log))
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToTestAccount(testAccount.address)

    const tokenAmount = 2222222222222222222222222n
    const tokenInfo = await mintToken(testAccount.address, tokenAmount)
    const balances0 = await nodeProvider.addresses.getAddressesAddressBalance(testAccount.address)

    const transferAmount = 2222222222222222222222222n / 2n
    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7',
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
          tokens: [{ id: tokenInfo.contractId, amount: transferAmount.toString() }]
        }
      ]
    })

    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    expect(transactionVerifySignature(buildTxResult.txId, testAccount.publicKey, signature)).toBe(true)

    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitTxConfirmed(nodeProvider, submitResult.txId, 1, 1000)
    const balances1 = await nodeProvider.addresses.getAddressesAddressBalance(testAccount.address)
    const gasFee = BigInt(buildTxResult.gasAmount) * BigInt(buildTxResult.gasPrice)
    const alphBalance = BigInt(balances1.balance)
    expect(alphBalance).toEqual(BigInt(balances0.balance) - gasFee - ONE_ALPH * 2n)

    const tokenBalance = balances1.tokenBalances!.find((t) => t.id === tokenInfo.contractId)!
    expect(tokenBalance.amount).toEqual((tokenAmount - transferAmount).toString())

    await app.close()
  }, 120000)

  it('should transfer to multisig address', async () => {
    const transport = await NodeTransport.open('')
    listen((log) => console.log(log))
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToTestAccount(testAccount.address)

    const tokenAmount = 2222222222222222222222222n
    const tokenInfo = await mintToken(testAccount.address, tokenAmount)
    const balances0 = await nodeProvider.addresses.getAddressesAddressBalance(testAccount.address)

    const transferAmount = 2222222222222222222222222n / 2n
    const multiSigAddress = 'X3KYVteDjsKuUP1F68Nv9iEUecnnkMuwjbC985AnA6MvciDFJ5bAUEso2Sd7sGrwZ5rfNLj7Rp4n9XjcyzDiZsrPxfhNkPYcDm3ce8pQ9QasNFByEufMi3QJ3cS9Vk6cTpqNcq';
    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: multiSigAddress,
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
          tokens: [{ id: tokenInfo.contractId, amount: transferAmount.toString() }]
        }
      ]
    })

    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    expect(transactionVerifySignature(buildTxResult.txId, testAccount.publicKey, signature)).toBe(true)

    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitTxConfirmed(nodeProvider, submitResult.txId, 1, 1000)
    const balances1 = await nodeProvider.addresses.getAddressesAddressBalance(testAccount.address)
    const gasFee = BigInt(buildTxResult.gasAmount) * BigInt(buildTxResult.gasPrice)
    const alphBalance = BigInt(balances1.balance)
    expect(alphBalance).toEqual(BigInt(balances0.balance) - gasFee - ONE_ALPH * 2n)

    const tokenBalance = balances1.tokenBalances!.find((t) => t.id === tokenInfo.contractId)!
    expect(tokenBalance.amount).toEqual((tokenAmount - transferAmount).toString())

    await app.close()
  }, 120000)
})

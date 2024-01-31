import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos'
import AlephiumApp, { GROUP_NUM } from '../src'
import blake from 'blakejs'
import fetch from 'node-fetch'
import { ALPH_TOKEN_ID, Account, Address, NodeProvider, ONE_ALPH, groupOfAddress, hexToBinUnsafe, transactionVerifySignature } from '@alephium/web3'
import { getSigner, transfer } from '@alephium/web3-test'
import { waitTxConfirmed } from '@alephium/cli'

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

async function pressButton(button: 'left' | 'right' | 'both') {
  await sleep(500)
  return fetch(`http://localhost:25000/button/${button}`, {
    method: 'POST',
    body: JSON.stringify({ action: 'press-and-release' })
  })
}

function getRandomInt(min, max) {
  min = Math.ceil(min)
  max = Math.floor(max)
  return Math.floor(Math.random() * (max - min) + min) // The maximum is exclusive and the minimum is inclusive
}

describe('sdk', () => {
  const apduPort = 9999
  const nodeProvider = new NodeProvider("http://127.0.0.1:22973")
  let pathIndex: number
  let path: string

  beforeEach(() => {
    pathIndex = getRandomInt(0, 1000000)
    path = `m/44'/1234'/0'/0/` + pathIndex
  })

  it('should get version', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const version = await app.getVersion()
    expect(version).toBe('0.2.0')
    await app.close()
  })

  it('should get public key', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [account, hdIndex] = await app.getAccount(path)
    expect(hdIndex).toBe(pathIndex)
    console.log(account)
    await app.close()
  })

  it('should get public key for group', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    Array(GROUP_NUM).forEach(async (_, group) => {
      const [account, hdIndex] = await app.getAccount(path, group)
      expect(hdIndex >= pathIndex).toBe(true)
      expect(groupOfAddress(account.address)).toBe(group)
      expect(account.keyType).toBe('default')
    })
    await app.close()
  })

  it('should get public key for group for Schnorr signature', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    Array(GROUP_NUM).forEach(async (_, group) => {
      await expect(app.getAccount(path, group, 'bip340-schnorr')).rejects.toThrow('BIP340-Schnorr is not supported yet')
    })
    await app.close()
  })

  async function transferToTestAccount(address: Address) {
    const fromAccount = await getSigner()
    const transferResult = await transfer(fromAccount, address, ALPH_TOKEN_ID, ONE_ALPH * 10n)
    await waitTxConfirmed(nodeProvider, transferResult.txId, 1, 1000)
    const balance0 = await getALPHBalance(address)
    expect(balance0).toEqual(ONE_ALPH * 10n)
  }

  async function getALPHBalance(address: Address) {
    const balances = await nodeProvider.addresses.getAddressesAddressBalance(address)
    return BigInt(balances.balance)
  }

  async function clickAndApprove(times: number) {
    for (let i = 0; i < times; i++) {
      await pressButton('right')
    }
    await pressButton('both')
  }

  it('should transfer alph to p2pkh address', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToTestAccount(testAccount.address)

    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [{
        address: '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7',
        attoAlphAmount: (ONE_ALPH * 5n).toString(),
      }]
    })

    function approve(index: number) {
      if (index >= 7) return
      if (index === 3) { // input
        setTimeout(async () => {
          await clickAndApprove(4)
          approve(index + 1)
        }, 1000)
      } else if (index >= 4) { // outputs and signature
        setTimeout(async () => {
          await clickAndApprove(5)
          approve(index + 1)
        }, 1000)
      } else {
        setTimeout(async () => {
          await clickAndApprove(2)
          approve(index + 1)
        }, 1000)
      }
    }

    approve(0)
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
  }, 60000)

  it('should transfer alph to multisig address', async () => {
    const nodeProvider = new NodeProvider("http://127.0.0.1:22973")
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToTestAccount(testAccount.address)

    const multiSigAddress = 'X3KYVteDjsKuUP1F68Nv9iEUecnnkMuwjbC985AnA6MvciDFJ5bAUEso2Sd7sGrwZ5rfNLj7Rp4n9XjcyzDiZsrPxfhNkPYcDm3ce8pQ9QasNFByEufMi3QJ3cS9Vk6cTpqNcq';
    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [{
        address: multiSigAddress,
        attoAlphAmount: (ONE_ALPH * 5n).toString(),
      }]
    })

    function approve(index: number) {
      if (index >= 7) return
      if (index === 3) { // input
        setTimeout(async () => {
          await clickAndApprove(4)
          approve(index + 1)
        }, 1000)
      } else if (index == 4) { // multi-sig output
        setTimeout(async () => {
          await clickAndApprove(10)
          approve(index + 1)
        }, 1000)
      } else if (index > 4) { // change output and signature
        setTimeout(async () => {
          await clickAndApprove(5)
          approve(index + 1)
        }, 1000)
      } else {
        setTimeout(async () => {
          await clickAndApprove(2)
          approve(index + 1)
        }, 1000)
      }
    }

    approve(0);
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
  }, 60000)
})

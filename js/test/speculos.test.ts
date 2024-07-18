import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos'
import AlephiumApp, { GROUP_NUM } from '../src'
import fetch from 'node-fetch'
import { ALPH_TOKEN_ID, Address, NodeProvider, ONE_ALPH, binToHex, codec, groupOfAddress, node, transactionVerifySignature, waitForTxConfirmation, web3 } from '@alephium/web3'
import { getSigner, mintToken, transfer } from '@alephium/web3-test'
import { PrivateKeyWallet } from '@alephium/web3-wallet'
import blake from 'blakejs'

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
  web3.setCurrentNodeProvider(nodeProvider)
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
    for (let group = 0; group < GROUP_NUM; group++) {
      const [account, hdIndex] = await app.getAccount(path, group)
      expect(hdIndex >= pathIndex).toBe(true)
      expect(groupOfAddress(account.address)).toBe(group)
      expect(account.keyType).toBe('default')
    }
    await app.close()
  })

  it('should get public key for group for Schnorr signature', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    for (let group = 0; group < GROUP_NUM; group++) {
      await expect(app.getAccount(path, group, 'bip340-schnorr')).rejects.toThrow('BIP340-Schnorr is not supported yet')
    }
    await app.close()
  })

  it('should sign hash', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)

    const [account] = await app.getAccount(path)
    console.log(account)

    const hash = Buffer.from(blake.blake2b(Buffer.from([0, 1, 2, 3, 4]), undefined, 32))
    setTimeout(async () => {
      await clickAndApprove(5)
    }, 1000)
    const signature = await app.signHash(path, hash)
    console.log(signature)
    await app.close()

    expect(transactionVerifySignature(hash.toString('hex'), account.publicKey, signature)).toBe(true)
  }, 10000)

  async function transferToAddress(address: Address, amount: bigint = ONE_ALPH * 10n) {
    const balance0 = await getALPHBalance(address)
    const fromAccount = await getSigner()
    const transferResult = await transfer(fromAccount, address, ALPH_TOKEN_ID, amount)
    await waitForTxConfirmation(transferResult.txId, 1, 1000)
    const balance1 = await getALPHBalance(address)
    expect(balance1 - balance0).toEqual(amount)
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
    await transferToAddress(testAccount.address)

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

    function approve(index: number) {
      if (index >= 6) return
      if (index >= 2) { // outputs and signature
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
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance1 = await getALPHBalance(testAccount.address)
    expect(balance1 < (ONE_ALPH * 5n)).toEqual(true)

    await app.close()
  }, 120000)

  it('should transfer alph to multisig address', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)

    const multiSigAddress = 'X3KYVteDjsKuUP1F68Nv9iEUecnnkMuwjbC985AnA6MvciDFJ5bAUEso2Sd7sGrwZ5rfNLj7Rp4n9XjcyzDiZsrPxfhNkPYcDm3ce8pQ9QasNFByEufMi3QJ3cS9Vk6cTpqNcq';
    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: multiSigAddress,
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
        },
        {
          address: multiSigAddress,
          attoAlphAmount: (ONE_ALPH * 3n).toString(),
        },
      ]
    })

    function approve(index: number) {
      if (index >= 6) return
      if (index == 2 || index == 3) { // multi-sig outputs
        setTimeout(async () => {
          await clickAndApprove(10)
          approve(index + 1)
        }, 1000)
      } else if (index > 3) { // change output and signature
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
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance1 = await getALPHBalance(testAccount.address)
    expect(balance1 < (ONE_ALPH * 5n)).toEqual(true)

    await app.close()
  }, 120000)

  it('should transfer token to multisig address', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)

    const tokenInfo = await mintToken(testAccount.address, 2222222222222222222222222n);

    const multiSigAddress = 'X3KYVteDjsKuUP1F68Nv9iEUecnnkMuwjbC985AnA6MvciDFJ5bAUEso2Sd7sGrwZ5rfNLj7Rp4n9XjcyzDiZsrPxfhNkPYcDm3ce8pQ9QasNFByEufMi3QJ3cS9Vk6cTpqNcq';
    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: multiSigAddress,
          attoAlphAmount: (ONE_ALPH * 5n).toString(),
          tokens: [
            {
              id: tokenInfo.contractId,
              amount: "1111111111111111111111111",
            }
          ]
        }
      ]
    })

    function approve(index: number) {
      if (index > 6) return
      if (index <= 1) {
        setTimeout(async () => {
          await clickAndApprove(2)
          approve(index + 1)
        }, 1000)
      } else if (index === 2) { // multi-sig token output
        setTimeout(async () => {
          await clickAndApprove(16)
          approve(index + 1)
        }, 1000)
      } else if (index === 3) { // multi-sig alph output
        setTimeout(async () => {
          await clickAndApprove(10)
          approve(index + 1)
        }, 1000)
      } else if (index === 4) { // token change output
        setTimeout(async () => {
          await clickAndApprove(11)
          approve(index + 1)
        }, 1000)
      } else if (index >= 5) { // alph change output and signature
        setTimeout(async () => {
          await clickAndApprove(5)
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
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balances = await nodeProvider.addresses.getAddressesAddressBalance(testAccount.address)
    const alphBalance = BigInt(balances.balance)
    expect(alphBalance < (ONE_ALPH * 5n)).toEqual(true)

    expect(balances.tokenBalances!.length).toEqual(1)
    const token = balances.tokenBalances![0]
    expect(token.id).toEqual(tokenInfo.contractId)
    expect(token.amount).toEqual('1111111111111111111111111')

    await app.close()
  }, 120000)

  it('should transfer from multiple inputs', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    for (let i = 0; i < 20; i += 1) {
      await transferToAddress(testAccount.address, ONE_ALPH)
    }

    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7',
          attoAlphAmount: (ONE_ALPH * 19n).toString(),
        }
      ]
    })

    function approve(index: number) {
      if (index >= 5) return
      if (index >= 2) { // outputs and signature
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
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance = await getALPHBalance(testAccount.address)
    expect(balance < (ONE_ALPH * 1n)).toEqual(true)

    await app.close()
  }, 120000)

  function getAccount(groupIndex: number): { account: PrivateKeyWallet, unlockScript: string } {
    const useDefaultKeyType = Math.random() >= 0.5
    if (useDefaultKeyType) {
      const account = PrivateKeyWallet.Random(groupIndex)
      return { account, unlockScript: '00' + account.publicKey }
    }

    const account = PrivateKeyWallet.Random(groupIndex, nodeProvider, 'bip340-schnorr')
    const unlockScript = '02' + `0101000000000458144020${account.publicKey}8685` + '00'
    return { account, unlockScript }
  }

  it('should transfer from different input addresses', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    const { account: newAccount, unlockScript: unlockScript0 } = getAccount(testAccount.group)
    for (let i = 0; i < 2; i += 1) {
      await transferToAddress(testAccount.address, ONE_ALPH)
      await transferToAddress(newAccount.address, ONE_ALPH)
    }

    const utxos0 = await nodeProvider.addresses.getAddressesAddressUtxos(newAccount.address)
    expect(utxos0.utxos.length).toEqual(2)
    const utxos1 = await nodeProvider.addresses.getAddressesAddressUtxos(testAccount.address)
    expect(utxos1.utxos.length).toEqual(2)

    const useSameAsPrevious = Math.random() >= 0.5
    const inputs0: node.AssetInput[] = utxos0.utxos.map((utxo, index) => {
      const unlockScript = index > 0 && useSameAsPrevious ? '03' : unlockScript0
      return { outputRef: utxo.ref, unlockScript }
    })
    const unlockScript1 = '00' + testAccount.publicKey
    const inputs1: node.AssetInput[] = utxos1.utxos.map((utxo, index) => {
      const unlockScript = index > 0 && useSameAsPrevious ? '03' : unlockScript1
      return { outputRef: utxo.ref, unlockScript }
    })
    const unsignedTx: node.UnsignedTx = {
      txId: '',
      version: 0,
      networkId: 4,
      gasAmount: 100000,
      gasPrice: (ONE_ALPH / 100000n).toString(),
      inputs: inputs0.concat(inputs1),
      fixedOutputs: [{
        hint: 0,
        key: '',
        attoAlphAmount: (ONE_ALPH * 3n).toString(),
        address: testAccount.address,
        tokens: [],
        lockTime: 0,
        message: ''
      }]
    }
    const txBytes = codec.unsignedTxCodec.encodeApiUnsignedTx(unsignedTx)
    const signResult0 = await newAccount.signUnsignedTx({
      signerAddress: newAccount.address,
      unsignedTx: binToHex(txBytes)
    })

    function approve(index: number) {
      if (index > 5) return
      if (index === 2 || index === 3) { // inputs
        setTimeout(async () => {
          await clickAndApprove(4)
          approve(index + 1)
        }, 1000)
      } else if (index >= 4) { // outputs and tx id
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
    const signature1 = await app.signUnsignedTx(path, Buffer.from(txBytes))
    expect(transactionVerifySignature(signResult0.txId, testAccount.publicKey, signature1)).toBe(true)

    const submitResult = await nodeProvider.multisig.postMultisigSubmit({
      unsignedTx: binToHex(txBytes),
      signatures: [signResult0.signature, signature1]
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance = await getALPHBalance(testAccount.address)
    expect(balance).toEqual(ONE_ALPH * 3n)

    await app.close()
  }, 120000)

  it('should test contract deployment', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)
    const buildTxResult = await nodeProvider.contracts.postContractsUnsignedTxDeployContract({
      fromPublicKey: testAccount.publicKey,
      bytecode: '00010c010000000002d38d0b3636020000'
    })

    function approve(index: number) {
      if (index > 3) return
      if (index === 2 || index === 3) {
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

    setTimeout(async () => await clickAndApprove(3))
    await expect(app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))).rejects.toThrow()

    async function enableBlindSigning() {
      await clickAndApprove(2)
    }

    await sleep(1000)
    await enableBlindSigning()
    approve(0)
    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)

    await app.close()
  }, 120000)
})

import { AlephiumApp, GROUP_NUM } from '../src/ledger-app'
import { ALPH_TOKEN_ID, Address, DUST_AMOUNT, NodeProvider, ONE_ALPH, binToHex, codec, groupOfAddress, node, sleep, transactionVerifySignature, waitForTxConfirmation, web3 } from '@alephium/web3'
import { getSigner, mintToken, transfer } from '@alephium/web3-test'
import { PrivateKeyWallet } from '@alephium/web3-wallet'
import blake from 'blakejs'
import { approveAddress, approveHash, approveTx, createTransport, enableBlindSigning, getRandomInt, isStaxOrFlex, needToAutoApprove, OutputType, skipBlindSigningWarning, staxFlexAcceptRisk, staxFlexApproveOnce } from './utils'
import { TokenMetadata } from '../src/types'
import { randomBytes } from 'crypto'
import { merkleTokens, tokenMerkleProofs } from '../src/merkle'

describe('ledger wallet', () => {
  const nodeProvider = new NodeProvider("http://127.0.0.1:22973")
  web3.setCurrentNodeProvider(nodeProvider)
  let pathIndex: number
  let path: string

  beforeEach(() => {
    pathIndex = getRandomInt(0, 1000000)
    path = `m/44'/1234'/0'/0/` + pathIndex
  })

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

  it('should get version', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const version = await app.getVersion()
    expect(version).toBe('0.4.2')
    await app.close()
  })

  it('should get public key', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [account, hdIndex] = await app.getAccount(path)
    expect(hdIndex).toBe(pathIndex)
    console.log(account)
    await app.close()
  })

  it('should get public key and confirm address', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    approveAddress()
    const [account, hdIndex] = await app.getAccount(path, undefined, undefined, true)
    expect(hdIndex).toBe(pathIndex)
    console.log(account)
    await app.close()
  }, 30000)

  it('should get public key for group', async () => {
    const transport = await createTransport()
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
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    for (let group = 0; group < GROUP_NUM; group++) {
      await expect(app.getAccount(path, group, 'bip340-schnorr')).rejects.toThrow('BIP340-Schnorr is not supported yet')
    }
    await app.close()
  })

  it('should sign hash', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)

    const [account] = await app.getAccount(path)
    console.log(account)

    const hash = Buffer.from(blake.blake2b(Buffer.from([0, 1, 2, 3, 4]), undefined, 32))
    approveHash()
    const signature = await app.signHash(path, hash)
    console.log(signature)
    await app.close()

    expect(transactionVerifySignature(hash.toString('hex'), account.publicKey, signature)).toBe(true)
  }, 10000)

  it('should transfer alph to one address', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)

    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7',
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
        }
      ]
    })

    approveTx([OutputType.Base])
    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    expect(transactionVerifySignature(buildTxResult.txId, testAccount.publicKey, signature)).toBe(true)

    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance = await getALPHBalance(testAccount.address)
    expect(balance < (ONE_ALPH * 8n)).toEqual(true)

    await app.close()
  }, 120000)

  it('should transfer alph to multiple addresses', async () => {
    const transport = await createTransport()
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
          address: '1F1fu6GjuN9yUVRFVcgQKWwiTg8RMzKFv1BZFDwFcfWJq',
          attoAlphAmount: (ONE_ALPH * 3n).toString(),
        },
      ]
    })

    approveTx(Array(2).fill(OutputType.Base))
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
    const transport = await createTransport()
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
        }
      ]
    })

    approveTx([OutputType.Multisig]);
    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    expect(transactionVerifySignature(buildTxResult.txId, testAccount.publicKey, signature)).toBe(true)

    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance1 = await getALPHBalance(testAccount.address)
    expect(balance1 < (ONE_ALPH * 8n)).toEqual(true)

    await app.close()
  }, 120000)

  it('should transfer token to multisig address', async () => {
    const transport = await createTransport()
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

    approveTx([OutputType.MultisigAndToken, OutputType.Multisig])
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

  async function genTokensAndDestinations(
    fromAddress: string,
    toAddress: string,
    mintAmount: bigint,
    transferAmount: bigint
  ) {
    const tokens: TokenMetadata[] = []
    const tokenSymbol = 'TestTokenABC'
    const destinations: node.Destination[] = []
    for (let i = 0; i < 5; i += 1) {
      const tokenInfo = await mintToken(fromAddress, mintAmount);
      const tokenMetadata: TokenMetadata = {
        version: 0,
        tokenId: tokenInfo.contractId,
        symbol: tokenSymbol.slice(0, tokenSymbol.length - i),
        decimals: 18 - i
      }
      tokens.push(tokenMetadata)
      destinations.push({
        address: toAddress,
        attoAlphAmount: DUST_AMOUNT.toString(),
        tokens: [
          {
            id: tokenMetadata.tokenId,
            amount: transferAmount.toString()
          }
        ]
      })
    }
    return { tokens, destinations }
  }

  it('should transfer tokens with proof', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)
    const newAccount = await getSigner()

    const selectedTokens = [
      merkleTokens[5], // decimals is 0
      merkleTokens[6], // decimals is 18
      merkleTokens[8], // decimals is 9
      merkleTokens[11], // decimals is 8
      merkleTokens[13], // decimals is 6
    ]
    const outputs: node.FixedAssetOutput[] = selectedTokens.map((token, index) => {
      return {
        hint:0,
        key: '',
        attoAlphAmount: DUST_AMOUNT.toString(),
        address: newAccount.address,
        tokens: [{ id: token.tokenId, amount: (BigInt(index + 1) * ONE_ALPH).toString() }],
        lockTime: 0,
        message: ''
      }
    })
    const unsignedTx: node.UnsignedTx = {
      txId: '',
      version: 0,
      networkId: 4,
      gasAmount: 100000,
      gasPrice: (ONE_ALPH / 10000000n).toString(),
      inputs: [{ outputRef: { hint: 0, key: binToHex(randomBytes(32)) }, unlockScript: '00' + testAccount.publicKey }],
      fixedOutputs: outputs
    }
    const encodedUnsignedTx = codec.unsignedTxCodec.encodeApiUnsignedTx(unsignedTx)

    approveTx(Array(5).fill(OutputType.BaseAndToken))
    const signature = await app.signUnsignedTx(path, Buffer.from(encodedUnsignedTx))
    const txId = blake.blake2b(encodedUnsignedTx, undefined, 32)
    expect(transactionVerifySignature(binToHex(txId), testAccount.publicKey, signature)).toBe(true)

    await app.close()
  }, 120000)

  it('should reject tx if the token proof is invalid', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)

    const toAddress = '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7'
    const selectedToken = merkleTokens[6] // the decimals is 18
    const output: node.FixedAssetOutput = {
      hint: 0,
      key: '',
      attoAlphAmount: DUST_AMOUNT.toString(),
      address: toAddress,
      tokens: [{ id: selectedToken.tokenId, amount: ONE_ALPH.toString() }],
      lockTime: 0,
      message: ''
    }
    const unsignedTx: node.UnsignedTx = {
      txId: '',
      version: 0,
      networkId: 4,
      gasAmount: 100000,
      gasPrice: (ONE_ALPH / 10000000n).toString(),
      inputs: [{ outputRef: { hint: 0, key: binToHex(randomBytes(32)) }, unlockScript: '00' + testAccount.publicKey }],
      fixedOutputs: [output]
    }
    const encodedUnsignedTx = codec.unsignedTxCodec.encodeApiUnsignedTx(unsignedTx)

    const originalProof = tokenMerkleProofs[selectedToken.tokenId]
    const invalidProof = originalProof.slice(0, originalProof.length - 64)
    tokenMerkleProofs[selectedToken.tokenId] = invalidProof
    await expect(app.signUnsignedTx(path, Buffer.from(encodedUnsignedTx))).rejects.toThrow()
    tokenMerkleProofs[selectedToken.tokenId] = originalProof

    await app.close()
  }, 120000)

  it('should reject tx if the metadata version is invalid', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)

    const toAddress = '1BmVCLrjttchZMW7i6df7mTdCKzHpy38bgDbVL1GqV6P7'
    const tokenIndex = 6
    const selectedToken = merkleTokens[tokenIndex]
    const output: node.FixedAssetOutput = {
      hint: 0,
      key: '',
      attoAlphAmount: DUST_AMOUNT.toString(),
      address: toAddress,
      tokens: [{ id: selectedToken.tokenId, amount: ONE_ALPH.toString() }],
      lockTime: 0,
      message: ''
    }
    const unsignedTx: node.UnsignedTx = {
      txId: '',
      version: 0,
      networkId: 4,
      gasAmount: 100000,
      gasPrice: (ONE_ALPH / 10000000n).toString(),
      inputs: [{ outputRef: { hint: 0, key: binToHex(randomBytes(32)) }, unlockScript: '00' + testAccount.publicKey }],
      fixedOutputs: [output]
    }
    const encodedUnsignedTx = codec.unsignedTxCodec.encodeApiUnsignedTx(unsignedTx)
    merkleTokens[tokenIndex] = { ...selectedToken, version: 1 }
    await expect(app.signUnsignedTx(path, Buffer.from(encodedUnsignedTx))).rejects.toThrow()
    merkleTokens[tokenIndex] = selectedToken

    await app.close()
  }, 120000)

  it('should transfer from multiple inputs', async () => {
    const transport = await createTransport()
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

    approveTx([OutputType.Base])
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

  it('should test external inputs', async () => {
    const transport = await createTransport()
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
        address: newAccount.address,
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

    approveTx([OutputType.Base], true)
    const signature1 = await app.signUnsignedTx(path, Buffer.from(txBytes))
    expect(transactionVerifySignature(signResult0.txId, testAccount.publicKey, signature1)).toBe(true)

    const submitResult = await nodeProvider.multisig.postMultisigSubmit({
      unsignedTx: binToHex(txBytes),
      signatures: [signResult0.signature, signature1]
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance = await getALPHBalance(newAccount.address)
    expect(balance).toEqual(ONE_ALPH * 3n)

    await app.close()
  }, 120000)

  it('should test self transfer tx', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)

    const buildTxResult = await nodeProvider.transactions.postTransactionsBuild({
      fromPublicKey: testAccount.publicKey,
      destinations: [
        {
          address: testAccount.address,
          attoAlphAmount: (ONE_ALPH * 2n).toString(),
        }
      ]
    })

    approveTx([])
    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    expect(transactionVerifySignature(buildTxResult.txId, testAccount.publicKey, signature)).toBe(true)

    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const balance = await getALPHBalance(testAccount.address)
    expect(balance > (ONE_ALPH * 9n)).toEqual(true)

    await app.close()
  }, 12000)

  it('should test script execution tx', async () => {
    const transport = await createTransport()
    const app = new AlephiumApp(transport)
    const [testAccount] = await app.getAccount(path)
    await transferToAddress(testAccount.address)
    const buildTxResult = await nodeProvider.contracts.postContractsUnsignedTxDeployContract({
      fromPublicKey: testAccount.publicKey,
      bytecode: '00010c010000000002d38d0b3636020000'
    })

    setTimeout(() => skipBlindSigningWarning(), 1000)
    await expect(app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))).rejects.toThrow()

    await enableBlindSigning()
    if (needToAutoApprove()) {
      if (isStaxOrFlex()) {
        staxFlexAcceptRisk().then(() => approveTx([]))
      } else {
        approveTx([])
      }
    } else {
      // waiting for blind signing setting to be enabled
      await sleep(20000)
    }
    const signature = await app.signUnsignedTx(path, Buffer.from(buildTxResult.unsignedTx, 'hex'))
    const submitResult = await nodeProvider.transactions.postTransactionsSubmit({
      unsignedTx: buildTxResult.unsignedTx,
      signature: signature
    })
    await waitForTxConfirmation(submitResult.txId, 1, 1000)
    const details = await nodeProvider.transactions.getTransactionsDetailsTxid(submitResult.txId)
    expect(details.scriptExecutionOk).toEqual(true)

    await app.close()
  }, 120000)
})

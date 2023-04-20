import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos'
import AlephiumApp, { GROUP_NUM } from '../src'
import blake from 'blakejs'
import fetch from 'node-fetch'
import { groupOfAddress, transactionVerifySignature } from '@alephium/web3'

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
    expect(version).toBe('0.1.0')
    await transport.close()
  })

  it('should get public key', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const [account, hdIndex] = await app.getAccount(path)
    expect(hdIndex).toBe(pathIndex)
    console.log(account)
    await transport.close()
  })

  it('should get public key for group', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    Array(GROUP_NUM).forEach(async (_, group) => {
      const [account, hdIndex] = await app.getAccount(path, group)
      expect(hdIndex >= pathIndex).toBe(true)
      expect(groupOfAddress(account.address)).toBe(group)
    })
    await transport.close()
  })

  it('should sign hash', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)

    const [account] = await app.getAccount(path)
    console.log(account)

    const hash = Buffer.from(blake.blake2b(Buffer.from([0, 1, 2, 3, 4]), undefined, 32))
    setTimeout(async () => {
      await pressButton('both') // review message
      await pressButton('both') // done review
      await pressButton('right') // select signing
      await pressButton('both') // done selection
    }, 1000)
    const signature = await app.signHash(path, hash)
    console.log(signature)
    await transport.close()

    expect(transactionVerifySignature(hash.toString('hex'), account.publicKey, signature)).toBe(true)
  }, 10000)
})

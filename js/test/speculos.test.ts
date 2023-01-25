import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos'
import NodeTransport from '@ledgerhq/hw-transport-node-hid'
import { listen } from '@ledgerhq/logs'
import AlephiumApp from '../src'
import blake from 'blakejs'
import fetch from 'node-fetch'
import { transactionVerifySignature } from '@alephium/web3'

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

async function pressButton(button: 'left' | 'right' | 'both') {
  return fetch(`http://localhost:25000/button/${button}`, {
    method: 'POST',
    body: JSON.stringify({ action: 'press-and-release' })
  })
  sleep(500)
}

describe('sdk', () => {
  const apduPort = 9999
  let path: string

  beforeEach(() => {
    path = `m/44'/1234'/0'/0/` + Math.floor(1000000)
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
    const account = await app.getAccount(path)
    console.log(account)
    await transport.close()
  })

  it('should sign hash', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)

    const account = await app.getAccount(path)
    console.log(account)

    const hash = Buffer.from(blake.blake2b(Buffer.from([0, 1, 2, 3, 4]), undefined, 32))
    setTimeout(async () => {
      await pressButton('left') // any button action to pass the welcome message
      await pressButton('both') // review message
      await pressButton('both') // done review
      await pressButton('right') // select signing
      await pressButton('both') // done selection
      await pressButton('right') // exit sign ui
    }, 1000)
    const signature = await app.signHash(path, hash)
    console.log(signature)
    await transport.close()

    expect(transactionVerifySignature(hash.toString('hex'), account.publicKey, signature)).toBe(true)
  }, 10000)

  // it('should test node', async () => {
  //   const transport = await NodeTransport.open('')
  //   listen((log) => console.log(log))
  //   const app = new AlephiumApp(transport)

  //   const account = await app.getAccount(path)
  //   console.log(`For real ${JSON.stringify(account)}`)
  // })
})

import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos'
import AlephiumApp from '../src'

describe('sdk', () => {
  const apduPort = 9999
  const path = `m/44'/1234'/0'/0/0`

  it('should get public key', async () => {
    const transport = await SpeculosTransport.open({ apduPort })
    const app = new AlephiumApp(transport)
    const account = await app.getAccount(path)
    console.log(account)
    await transport.close()
  })
})

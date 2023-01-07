import { serializePath, splitPath } from './serde'

describe('serde', () => {
  it('should split path', () => {
    expect(splitPath(`m/44'/1234'/0'/0/0`)).toStrictEqual([44 + 0x80000000, 1234 + 0x80000000, 0 + 0x80000000, 0, 0])
    expect(splitPath(`44'/1234'/0'/0/0`)).toStrictEqual([44 + 0x80000000, 1234 + 0x80000000, 0 + 0x80000000, 0, 0])
  })

  it('should encode path', () => {
    expect(serializePath(`m/1'/2'/0'/0/0`)).toStrictEqual(
      Buffer.from([0x80, 0, 0, 1, 0x80, 0, 0, 2, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    )
    expect(serializePath(`m/1'/2'/0'/0/0`)).toStrictEqual(
      Buffer.from([0x80, 0, 0, 1, 0x80, 0, 0, 2, 0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    )
  })
})

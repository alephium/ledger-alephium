import { binToHex } from '@alephium/web3'
import { MAX_TOKEN_SYMBOL_LENGTH, serializePath, serializeTokenMetadata, splitPath, TokenMetadata } from './serde'
import { randomBytes } from 'crypto'

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

  it('should encode token metadata', () => {
    const token0: TokenMetadata = {
      version: 0,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'Token0',
      decimals: 8
    }
    const token1: TokenMetadata = {
      version: 1,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'Token1',
      decimals: 18
    }
    const token2: TokenMetadata = {
      version: 2,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'Token2',
      decimals: 6
    }
    const token3: TokenMetadata = {
      version: 3,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'Token3',
      decimals: 0
    }
    const token4: TokenMetadata = {
      version: 4,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'Token4',
      decimals: 12
    }

    const encodeSymbol = (symbol: string) => {
      return binToHex(Buffer.from(symbol, 'ascii')).padEnd(MAX_TOKEN_SYMBOL_LENGTH * 2, '0')
    }

    expect(binToHex(serializeTokenMetadata([]))).toEqual('00')
    expect(binToHex(serializeTokenMetadata([token0]))).toEqual(
      '01' + '00' + token0.tokenId + encodeSymbol(token0.symbol) + '08'
    )
    expect(binToHex(serializeTokenMetadata([token1]))).toEqual(
      '01' + '01' + token1.tokenId + encodeSymbol(token1.symbol) + '12'
    )
    expect(binToHex(serializeTokenMetadata([token0, token1]))).toEqual(
      '02' + '00' + token0.tokenId + encodeSymbol(token0.symbol) + '08' +
      '01' + token1.tokenId + encodeSymbol(token1.symbol) + '12'
    )
    expect(binToHex(serializeTokenMetadata([token0, token1, token2, token3, token4]))).toEqual(
      '05' + '00' + token0.tokenId + encodeSymbol(token0.symbol) + '08' +
      '01' + token1.tokenId + encodeSymbol(token1.symbol) + '12' +
      '02' + token2.tokenId + encodeSymbol(token2.symbol) + '06' +
      '03' + token3.tokenId + encodeSymbol(token3.symbol) + '00' +
      '04' + token4.tokenId + encodeSymbol(token4.symbol) + '0c'
    )

    expect(() => serializeTokenMetadata([token0, token1, token0])).toThrow(
      'There are duplicate tokens'
    )

    const token5: TokenMetadata = {
      version: 5,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'Token5',
      decimals: 18
    }
    expect(() => serializeTokenMetadata([token0, token1, token2, token3, token4, token5])).toThrow(
      'The token size exceeds maximum size'
    )

    const invalidToken: TokenMetadata = {
      ...token0,
      tokenId: binToHex(randomBytes(33))
    }
    expect(() => serializeTokenMetadata([token0, invalidToken])).toThrow('Invalid token id')

    const longSymbolToken: TokenMetadata = {
      ...token0,
      tokenId: binToHex(randomBytes(32)),
      symbol: 'LongSymbolToken'
    }
    expect(() => serializeTokenMetadata([token0, longSymbolToken, token1])).toThrow(
      'The token symbol is too long'
    )
  })
})

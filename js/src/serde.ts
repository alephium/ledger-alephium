import { isHexString } from "@alephium/web3"

export const TRUE = 0x10
export const FALSE = 0x00

export function splitPath(path: string): number[] {
  const result: number[] = []
  const allComponents = path.trim().split('/')
  const components = allComponents.length > 0 && allComponents[0] == 'm' ? allComponents.slice(1) : allComponents
  components.forEach((element) => {
    let number = parseInt(element, 10)
    if (isNaN(number)) {
      throw Error(`Invalid bip32 path: ${path}`)
    }
    if (element.length > 1 && element[element.length - 1] === "'") {
      number += 0x80000000
    }
    result.push(number)
  })
  return result
}

export function serializePath(path: string): Buffer {
  const nodes = splitPath(path)

  if (nodes.length != 5) {
    throw Error('Invalid BIP32 path length')
  }
  const buffer = Buffer.alloc(nodes.length * 4)
  nodes.forEach((element, index) => buffer.writeUInt32BE(element, 4 * index))
  return buffer
}

const MAX_TOKEN_SIZE = 5
const TOKEN_METADATA_SIZE = 42
export interface TokenMetadata {
  version: number,
  tokenId: string,
  symbol: string,
  decimals: number
}

function symbolToBytes(symbol: string): Buffer {
  const buffer = Buffer.alloc(8, 0)
  for (let i = 0; i < symbol.length; i++) {
    buffer[i] = symbol.charCodeAt(i) & 0xFF
  }
  return buffer
}

function check(tokens: TokenMetadata[]) {
  const hasDuplicate = tokens.some((token, index) => index !== tokens.findIndex((t) => t.tokenId === token.tokenId))
  if (hasDuplicate) {
    throw new Error(`There are duplicate tokens`)
  }

  tokens.forEach((token) => {
    if (!(isHexString(token.tokenId) && token.tokenId.length === 64)) {
      throw new Error(`Invalid token id: ${token.tokenId}`)
    }
    if (token.symbol.length > 8) {
      throw new Error(`The token symbol is too long: ${token.symbol}`)
    }
  })

  if (tokens.length > 5) {
    throw new Error(`The token size exceeds maximum size`)
  }
}

export function serializeTokenMetadata(tokens: TokenMetadata[]): Buffer {
  check(tokens)
  const array = tokens
    .map((metadata) => {
      const symbolBytes = symbolToBytes(metadata.symbol)
      const buffer = Buffer.concat([
        Buffer.from([metadata.version]),
        Buffer.from(metadata.tokenId, 'hex'),
        symbolBytes,
        Buffer.from([metadata.decimals])
      ])
      if (buffer.length !== TOKEN_METADATA_SIZE) {
        throw new Error(`Invalid token metadata: ${metadata}`)
      }
      return buffer
    })
  return Buffer.concat([Buffer.from([array.length]), ...array])
}

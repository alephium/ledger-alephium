import { binToHex, hexToBinUnsafe } from '@alephium/web3'
import mainnetTokenListJson from '../merkle-tree/token.json'
import proofsJson from '../merkle-tree/proofs.json'
import { serializeSingleTokenMetadata } from './serde'
import { TokenMetadata } from './types'
import { blake2b } from 'blakejs'

export const merkleTokens: TokenMetadata[] = mainnetTokenListJson.tokens.map((token) => {
  return {
    version: 0,
    tokenId: token.id,
    symbol: token.symbol,
    decimals: token.decimals
  }
})

export function hashPair(a: Uint8Array, b: Uint8Array): Uint8Array {
  return blake2b(Buffer.concat([a, b].sort(Buffer.compare)), undefined, 32)
}

function generateMerkleTree(tokens: TokenMetadata[]): Uint8Array[][] {
  let level: Uint8Array[] = tokens.map((token) => blake2b(serializeSingleTokenMetadata(token), undefined, 32))

  const tree: Uint8Array[][] = []
  while (level.length > 1) {
    tree.push(level)
    level = level.reduce<Uint8Array[]>((acc, _, i, arr) => {
      if (i % 2 === 0) {
        acc.push(i + 1 < arr.length ? hashPair(arr[i], arr[i + 1]) : arr[i])
      }
      return acc
    }, [])
  }
  tree.push(level) // Root

  return tree
}

export function generateProofs(): { proofs: Record<string, string>; root: string } {
  const tree = generateMerkleTree(merkleTokens)
  const proofs = merkleTokens.reduce<Record<string, string>>((acc, token, tokenIndex) => {
    const proof = tree.slice(0, -1).reduce<Uint8Array[]>((proofAcc, level, levelIndex) => {
      const index = Math.floor(tokenIndex / 2 ** levelIndex)
      const pairIndex = index % 2 === 0 ? index + 1 : index - 1
      const siblingOrUncle = level[pairIndex]

      if (siblingOrUncle) {
        proofAcc.push(siblingOrUncle)
      }

      return proofAcc
    }, [])

    acc[token.tokenId] = proof.map((hash) => binToHex(hash)).join('')
    return acc
  }, {})

  console.log('root', tree[tree.length - 1].map((hash) => binToHex(hash)).join(''))
  return { proofs, root: binToHex(tree[tree.length - 1][0]) }
}

export const tokenMerkleRoot = hexToBinUnsafe('b3380866c595544781e9da0ccd79399de8878abfb0bf40545b57a287387d419d')
export const tokenMerkleProofs = proofsJson as Record<string, string>

import { hashPair, merkleTokens, tokenMerkleProofs, tokenMerkleRoot } from '../src/merkle'
import { serializeSingleTokenMetadata } from '../src/serde'
import { blake2b } from 'blakejs'
import { binToHex } from '@alephium/web3'

describe('Merkle', () => {
  it('should verify proofs', () => {
    for (const token of merkleTokens) {
      const proof = tokenMerkleProofs[token.tokenId]

      let currentHash = blake2b(serializeSingleTokenMetadata(token), undefined, 32)
      for (let i = 0; i < proof.length; i += 64) {
        const sibling = proof.slice(i, i + 64)
        currentHash = hashPair(currentHash, Buffer.from(sibling, 'hex'))
      }

      expect(JSON.stringify(currentHash)).toBe(JSON.stringify(tokenMerkleRoot))
    }
  })
})

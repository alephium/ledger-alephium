import { merkleTokens, tokenMerkleProofs } from '../src/merkle'
import { assert, encodeProofLength, encodeTokenMetadata, encodeUnsignedTx } from '../src/tx-encoder'
import { MAX_PAYLOAD_SIZE, MAX_TOKEN_SIZE, TOKEN_METADATA_SIZE } from '../src'
import { serializePath, serializeSingleTokenMetadata } from '../src/serde';
import { randomBytes } from 'crypto';

describe('TxEncoder', () => {

  function shuffle<T>(array: T[]): T[] {
    for (let i = array.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [array[i], array[j]] = [array[j], array[i]]
    }
    return array
  }

  function getFrameSize(proofLength: number): number {
    if (proofLength <= 192) return 1
    if (proofLength <= 416) return 2
    if (proofLength <= 640) return 3
    throw Error(`Invalid proof length: ${proofLength}`)
  }

  it('should encode token metadata and proof', () => {
    const frames0 = encodeTokenMetadata([])
    expect(frames0).toEqual([{ p1: 0, p2: 0, data: Buffer.from([0]) }])

    const tokenSize = Math.floor(Math.random() * MAX_TOKEN_SIZE) + 1
    assert(tokenSize >= 1 && tokenSize <= 5, 'Invalid token size')
    const tokens = shuffle(Object.entries(tokenMerkleProofs))
    const selectedTokens = tokens.slice(0, tokenSize)
    const tokenMetadatas = selectedTokens.map(([tokenId]) => merkleTokens.find((t) => t.tokenId === tokenId)!)
    const frames = encodeTokenMetadata(tokenMetadatas)
    const tokenAndProofs = Buffer.concat(frames.map((frame, index) => index === 0 ? frame.data.slice(1) : frame.data))

    const expected = Buffer.concat(tokenMetadatas.map((metadata, index) => {
      const proof = Buffer.from(selectedTokens[index][1], 'hex')
      const encodedProofLength = encodeProofLength(proof.length)
      const encodedTokenMetadata = serializeSingleTokenMetadata(metadata)
      return Buffer.concat([encodedTokenMetadata, encodedProofLength, proof])
    }))

    expect(tokenAndProofs).toEqual(expected)

    let frameIndex = 0
    tokenMetadatas.forEach((_, index) => {
      const proof = Buffer.from(selectedTokens[index][1], 'hex')
      const isFirstToken = index === 0
      const prefixLength = isFirstToken ? 1 + TOKEN_METADATA_SIZE + 2 : TOKEN_METADATA_SIZE + 2
      const tokenFrames = frames.slice(frameIndex, frameIndex + getFrameSize(proof.length))
      const firstFrameP2 = isFirstToken ? 0 : 1
      expect(tokenFrames[0].p1).toEqual(0)
      expect(tokenFrames[0].p2).toEqual(firstFrameP2)
      tokenFrames.slice(1).forEach((frame) => {
        expect(frame.p1).toEqual(0)
        expect(frame.p2).toEqual(2)
      })

      const expectedProof = Buffer.concat([tokenFrames[0].data.slice(prefixLength), ...tokenFrames.slice(1).map((f) => f.data)])
      expect(proof).toEqual(expectedProof)

      frameIndex += tokenFrames.length
    })
  })

  it('should encode tx', () => {
    const path = `m/44'/1234'/0'/0/0`
    const encodedPath = serializePath(path)
    const unsignedTx0 = randomBytes(200)
    const frames0 = encodeUnsignedTx(path, unsignedTx0)
    expect(frames0).toEqual([{ p1: 1, p2: 0, data: Buffer.concat([encodedPath, unsignedTx0]) }])

    const unsignedTx1 = randomBytes(250)
    const frames1 = encodeUnsignedTx(path, unsignedTx1)
    expect(frames1).toEqual([
      { p1: 1, p2: 0, data: Buffer.concat([encodedPath, unsignedTx1.slice(0, MAX_PAYLOAD_SIZE - 20)]) },
      { p1: 1, p2: 1, data: unsignedTx1.slice( MAX_PAYLOAD_SIZE - 20) },
    ])
  })
})
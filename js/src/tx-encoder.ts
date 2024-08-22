import { tokenMerkleProofs } from "./merkle"
import { checkTokenMetadata, serializePath, serializeSingleTokenMetadata } from "./serde"
import { MAX_PAYLOAD_SIZE, TokenMetadata } from "./types"

export interface Frame {
  p1: number
  p2: number
  data: Buffer
}

export function encodeTokenMetadata(tokenMetadata: TokenMetadata[]): Frame[] {
  const frames = tokenMetadata.flatMap((metadata, index) => {
    const isFirstToken = index === 0
    const firstFramePrefix = isFirstToken ? Buffer.from([tokenMetadata.length]) : Buffer.alloc(0)
    const buffers = encodeTokenAndProof(metadata, firstFramePrefix)
    if (buffers.length === 0) return []

    assert(buffers.every((buffer) => buffer.length <= MAX_PAYLOAD_SIZE), 'Invalid token frame size')
    const frames: Frame[] = []
    const firstFrameP1 = isFirstToken ? 0 : 1
    frames.push({ p1: firstFrameP1, p2: 0, data: buffers[0] })
    buffers.slice(1).forEach((data) => frames.push({ p1: 1, p2: 1, data }))
    return frames
  })
  if (frames.length === 0) {
    return [{ p1: 0, p2: 0, data: Buffer.from([0]) }]
  } else {
    return frames
  }
}

function encodeTokenAndProof(
  tokenMetadata: TokenMetadata,
  firstFramePrefix: Buffer
): Buffer[] {
  const proof = tokenMerkleProofs[tokenMetadata.tokenId]
  if (proof === undefined) return []
  const proofBytes = Buffer.from(proof, 'hex')
  const encodedProofLength = encodeProofLength(proofBytes.length)
  const encodedTokenMetadata = serializeSingleTokenMetadata(tokenMetadata)

  const firstFrameRemainSize =
    MAX_PAYLOAD_SIZE - encodedTokenMetadata.length - encodedProofLength.length - firstFramePrefix.length
  const firstFrameProofSize = Math.floor(firstFrameRemainSize / 32) * 32
  if (firstFrameProofSize >= proofBytes.length) {
    return [Buffer.concat([firstFramePrefix, encodedTokenMetadata, encodedProofLength, proofBytes])]
  }

  const firstFrameProof = proofBytes.slice(0, firstFrameProofSize)
  const result: Buffer[] = [Buffer.concat([firstFramePrefix, encodedTokenMetadata, encodedProofLength, firstFrameProof])]
  let from_index = firstFrameProofSize
  while (from_index < proofBytes.length) {
    const remainProofLength = proofBytes.length - from_index
    const frameProofSize = Math.min(Math.floor(MAX_PAYLOAD_SIZE / 32) * 32, remainProofLength)
    const frameProof = proofBytes.slice(from_index, from_index + frameProofSize)
    from_index += frameProofSize
    result.push(frameProof)
  }
  return result
}

export function encodeProofLength(length: number): Uint8Array {
  assert((length % 32 === 0) && (length > 0 && length < 0xffff), 'Invalid token proof size')
  const buffer = Buffer.alloc(2);
  buffer.writeUint16BE(length);
  return buffer;
}

export function encodeUnsignedTx(path: string, unsignedTx: Buffer): Frame[] {
  const encodedPath = serializePath(path)
  const firstFrameTxLength = MAX_PAYLOAD_SIZE - 20;
  if (firstFrameTxLength >= unsignedTx.length) {
    return [{ p1: 2, p2: 0, data: Buffer.concat([encodedPath, unsignedTx]) }]
  }

  const firstFrameTxData = unsignedTx.slice(0, firstFrameTxLength)
  const frames: Frame[] = [{ p1: 2, p2: 0, data: Buffer.concat([encodedPath, firstFrameTxData]) }]
  let fromIndex = firstFrameTxLength
  while (fromIndex < unsignedTx.length) {
    const remain = unsignedTx.length - fromIndex
    const frameTxLength = Math.min(MAX_PAYLOAD_SIZE, remain)
    frames.push({ p1: 2, p2: 1, data: unsignedTx.slice(fromIndex, fromIndex + frameTxLength) })
    fromIndex += frameTxLength
  }
  return frames
}

export function assert(condition: boolean, msg: string) {
  if (!condition) throw Error(msg)
}

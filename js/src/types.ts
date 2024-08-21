export const MAX_TOKEN_SIZE = 5
export const MAX_TOKEN_SYMBOL_LENGTH = 12
export const TOKEN_METADATA_SIZE = 46

export interface TokenMetadata {
  version: number,
  tokenId: string,
  symbol: string,
  decimals: number
}

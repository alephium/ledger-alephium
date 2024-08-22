export const MAX_TOKEN_SIZE = 5
export const MAX_TOKEN_SYMBOL_LENGTH = 12
export const TOKEN_METADATA_SIZE = 46
// The maximum payload size is 255: https://github.com/LedgerHQ/ledger-live/blob/develop/libs/ledgerjs/packages/hw-transport/src/Transport.ts#L261
export const MAX_PAYLOAD_SIZE = 255

export interface TokenMetadata {
  version: number,
  tokenId: string,
  symbol: string,
  decimals: number
}

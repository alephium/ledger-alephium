# Alephium application: Technical Specifications

## About

This application describes the APDU messages interface to communicate with the Alephium application.

The application covers the following functionalities:

* Retrieve an address and public key given a BIP 32 path and group number
* Sign a transaction given a BIP 32 path

## APDUs

### GET PUBLIC KEY

#### Description

This command returns the public key and Alephium address for the given BIP 32 path.

#### Coding

*Command*
|  CLA  |  INS |       P1      |       P2                  |
|-------|------|---------------|---------------------------|
|  80   |  01  |     0 or 4    |  Any value between [0-3]  |

Alephium is a sharded blockchain. Currently, there are 4 groups in Alephium.
Users can choose any group's address. We use `P1` to represent the group
number(currently 4) and `P2` to represent the target group of the user
address.

If both `P1` and `P2` are 0, it means that no target group has been
specified. If `P2` is specified, the value of `P2` must be less than `P1`.

*Input Data*
| Length | Description |
|--------|-------------|
| `4`    | First derivation index (big endian) |
| `4`    | Second derivation index (big endian) |
| `4`    | Third derivation index (big endian) |
| `4`    | Fourth derivation index (big endian) |
| `4`    | Fifth derivation index (big endian) |

*Output Data*
| Length  | Description |
|---------|-------------|
| `65`    | Public key |
| `4`     | Derivation index (big endian) |

If the user specifies a target group(`P2`), the fifth derivation index will
serve as the starting index. It will increment by 1 with each iteration until
an address satisfying the target group is found. The final derivation index
will be returned along with the public key.

If the user does not specify a target group, the original derivation index is returned.

### SIGN UNSIGNED TX

#### Description

This command returns the signature for the given BIP 32 path and unsigned tx.

#### Coding

*Command*
|  CLA  |  INS |         P1            |     P2     |
|-------|------|-----------------------|------------|
|  80   |  02  |   0: first chunk      |  Not used  |
|       |      |   1: following chunk  |            |

*Input Data*

If P1 == first chunk

| Length     | Description |
|------------|-------------|
| `20`       | Derivation indexes |
| `variable` | Transaction payload |

If P2 == following chunk

| Length     | Description |
|------------|-------------|
| `variable` | Transaction payload |

*Output Data*
| Length     | Description |
|------------|-------------|
| `variable` | DER-encoded signature  |

## Status Words

The following standard status words are returned for all APDUs.

| SW     | SW name                  | Description |
|--------|--------------------------|-------------|
| 0x9000 | `Ok`                     | Success |
| 0x6E00 | `BadCla`                 | Invalid `CLA` |
| 0x6E01 | `BadIns`                 | Invalid `INS` |
| 0x6E02 | `BadP1P2`                | Invalid `P1` or `P2` |
| 0x6E04 | `UserCancelled`          | Rejected by user |
| 0xE000 | `TxDecodeFail`           | Failed to decode tx |
| 0xE001 | `TxSignFail`             | Failed to sign tx |
| 0xE002 | `Overflow`               | Stack overflow |
| 0xE003 | `DerivePathDecodeFail`   | Failed to decode derive path |
| 0xE004 | `BlindSigningNotEnabled` | Blind signing is not enabled |
| 0xEF00 | `InternalError`          | Internal error |

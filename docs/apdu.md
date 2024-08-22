# APDU protocol description

This document aims to provide a description of the APDU protocol supported by the app, explaining what each instruction does, the expected parameters and return values

## General Structure

The general structure of a request and response is as followed:

### Request / Command

| Field   | Type     | Content                | Note                   |
|:--------|:---------|:-----------------------|------------------------|
| CLA     | byte (1) | Application Identifier | 0x80                   |
| INS     | byte (1) | Instruction ID         |                        |
| P1      | byte (1) | Parameter 1            |                        |
| P2      | byte (1) | Parameter 2            |                        |
| L       | byte (1) | Bytes in payload       |                        |
| PAYLOAD | byte (L) | Payload                |                        |

### Response

| Field   | Type     | Content     | Note                     |
| ------- | -------- | ----------- | ------------------------ |
| ANSWER  | byte (?) | Answer      | depends on the command   |
| SW1-SW2 | byte (2) | Return code | see list of return codes |

#### Return codes

| Return code | Description               |
| ----------- | --------------------------|
| 0x9000      | Success                   |
| 0x6E00      | Bad CLA                   |
| 0x6E01      | Bad Ins                   |
| 0x6E02      | Bad P1/P2                 |
| 0x6E04      | User Cancelled            |
| 0xE000      | Failed to decode tx       |
| 0xE001      | Failed to sign tx         |
| 0xE002      | Stack overflow            |
| 0xE003      | Failed to decode path     |
| 0xE004      | Blind signing is disabled |
| 0xE005      | Failed to derive pub key  |
| 0xE006      | Invalid token size        |
| 0xE007      | Invalid metadata version  |
| 0xE008      | Invalid token proof size  |
| 0xEF00      | Internal error            |

## Commands definitions

### GetVersion

This command will return the app version

#### Command

| Field | Type     | Content                | Expected |
|-------|----------|------------------------|----------|
| CLA   | byte (1) | Application Identifier | 0x80     |
| INS   | byte (1) | Instruction ID         | 0x00     |
| P1    | byte (1) | Parameter 1            | ignored  |
| P2    | byte (1) | Parameter 2            | ignored  |
| L     | byte (1) | Bytes in payload       | 0        |

#### Response

| Field     | Type     | Content          | Note                            |
| --------- | -------- | ---------------- | ------------------------------- |
| MAJOR     | byte (1) | Version Major    |                                 |
| MINOR     | byte (1) | Version Minor    |                                 |
| PATCH     | byte (1) | Version Patch    |                                 |
| SW1-SW2   | byte (2) | Return code      | see list of return codes        |

### GetPubKey

This command returns the public key corresponding to the secret key found at the given path

#### Command

| Field   | Type     | Content                   | Expected        |
|---------|----------|---------------------------|-----------------|
| CLA     | byte (1) | Application Identifier    | 0x8A            |
| INS     | byte (1) | Instruction ID            | 0x01            |
| P1      | byte (1) | Parameter 1               | 0 or 4          |
| P2      | byte (1) | Parameter 2               | Any value between 0 and 3, inclusive |
| L       | byte (1) | Bytes in payload          | 0x15            |
| Path[0] | byte (4) | Derivation Path Data      | ?               |
| Path[1] | byte (4) | Derivation Path Data      | ?               |
| Path[2] | byte (4) | Derivation Path Data      | ?               |
| Path[3] | byte (4) | Derivation Path Data      | ?               |
| Path[4] | byte (4) | Derivation Path Data      | ?               |
| Flag    | byte (1) | Whether confirmation is needed | If not 0, display address and confirm before returning |

#### Response

| Field      | Type      | Content           | Note                     |
| ---------- | --------- | ----------------- | ------------------------ |
| PKEY       | byte (65) | Public key bytes  |                          |
| HD INDEX   | byte (4)  | Derivation index  |                          |
| SW1-SW2    | byte (2)  | Return code       | see list of return codes |

### SignHash

This command returns a signature of the passed hash

#### Command

| Field   | Type     | Content                   | Expected        |
|---------|----------|---------------------------|-----------------|
| CLA     | byte (1) | Application Identifier    | 0x8A            |
| INS     | byte (1) | Instruction ID            | 0x02            |
| P1      | byte (1) | Parameter 1               | ignored         |
| P2      | byte (1) | Parameter 2               | ignored         |
| L       | byte (1) | Bytes in payload          | 0x34            |
| Path[0] | byte (4) | Derivation Path Data      | ?               |
| Path[1] | byte (4) | Derivation Path Data      | ?               |
| Path[2] | byte (4) | Derivation Path Data      | ?               |
| Path[3] | byte (4) | Derivation Path Data      | ?               |
| Path[4] | byte (4) | Derivation Path Data      | ?               |
| Hash    | byte (32)| Hash                      | ?               |

#### Response

| Field    | Type      | Content     | Note                                  |
|----------|-----------|-------------|---------------------------------------|
| SIG      | byte (?)  | Signature   | DER-encoded signature                 |
| SW1-SW2  | byte (2)  | Return code | see list of return codes              |

### SignTx

This command returns a signature of the passed transaction

| Field | Type     | Content                     | Expected          |
|-------|----------|-----------------------------|-------------------|
| CLA   | byte (1) | Application Identifier      | 0x80              |
| INS   | byte (1) | Instruction ID              | 0x03              |
| P1    | byte (1) | Payload desc                | 0x00, 0x01, 0x02  |
| P2    | byte (1) | ignored                     | 0x00, 0x01        |
| L     | byte (1) | Bytes in payload            | (depends)         |

We use `P1` and `P2` to distinguish between different data frames:

* `P1` = 0 and `P2` = 0 indicates the first token metadata data frame
* `P1` = 1 and `P2` = 1 indicates a token proof data frame
* `P1` = 1 and `P2` = 0 indicates subsequent metadata data frames
* `P1` = 2 and `P2` = 0 indicates the first tx data frame
* `P1` = 2 and `P2` = 1 indicates subsequent tx data frames

Input data (first token metadata frame):

| Field          | Type                   | Content              | Expected          |
|----------------|------------------------|----------------------|-------------------|
| Token Size     | byte (1)               | Token Size           | Any value between 0 and 5, inclusive |
| Token Metadata | 46                     | Token Metadata       | ?                 |
| Token Proof Size |  2                   | Token Proof Size     | ?                 |
| Token Proof    | byte (?)               | Token Proof          | ?                 |

Input data (token proof frame):

| Field          | Type                   | Content              | Expected          |
|----------------|------------------------|----------------------|-------------------|
| Token Proof    | byte (?)               | Token Proof          | ?                 |

Input data (subsequent token metadata frame):

| Field          | Type                   | Content              | Expected          |
|----------------|------------------------|----------------------|-------------------|
| Token Metadata | 46                     | Token Metadata       | ?                 |
| Token Proof Size |  2                   | Token Proof Size     | ?                 |
| Token Proof    | byte (?)               | Token Proof          | ?                 |

Input data (first transaction data frame):

| Field          | Type                   | Content              | Expected          |
|----------------|------------------------|----------------------|-------------------|
| Path[0]        | byte (4)               | Derivation Path Data | ?                 |
| Path[1]        | byte (4)               | Derivation Path Data | ?                 |
| Path[2]        | byte (4)               | Derivation Path Data | ?                 |
| Path[3]        | byte (4)               | Derivation Path Data | ?                 |
| Path[4]        | byte (4)               | Derivation Path Data | ?                 |
| Payload        | byte (?)               | Transaction Payload  | ?                 |

Input data (subsequent transaction data frame):

| Field   | Type     | Content                   | Expected          |
|---------|----------|---------------------------|-------------------|
| Payload | byte (?) | Transaction payload       | ?                 |

#### Response

| Field    | Type      | Content     | Note                                  |
|----------|-----------|-------------|---------------------------------------|
| SIG      | byte (?)  | Signature   | DER-encoded signature                 |
| SW1-SW2  | byte (2)  | Return code | see list of return codes              |

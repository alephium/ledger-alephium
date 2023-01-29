# Ledger Alephium Nano App

This project is still under active development.

## Install

All of the release files are in the folder `release`. To build everything from source code, please refer to the next section.

You will need to install the tool `ledgerctl` to load Alphium app. The official guide is here: [https://github.com/LedgerHQ/ledgerctl#quick-install](https://github.com/LedgerHQ/ledgerctl#quick-install).

To install the app for Nano S:

```
cd release && ledgerctl install nanos.json
```

To install the app for Nano S+:

```
cd release && ledgerctl install nanosplus.json
```

To install the app for Nano X:

```
cd release && ledgerctl install nanox.json
```

## Uninstall

To uninstall the app:
```
ledgerctl delete Alephium
```

## Build from source

The following command will build the releases for Ledger Nano.

```shell
make app-builder-image
make release
```

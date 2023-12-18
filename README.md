# Ledger Alephium Nano App

This project is still under active development.

## Install

All of the release files are in the root folder. To build everything from source code, please refer to the next section.

You will need to install the tool `ledgerctl` to load Alphium app. The official guide is here: [https://github.com/LedgerHQ/ledgerctl#quick-install](https://github.com/LedgerHQ/ledgerctl#quick-install).

To install the app for Nano S:

```
ledgerctl install nanos.json
```

To install the app for Nano S+:

```
ledgerctl install nanosplus.json
```

Nano X manual installation isn't supported. The device no longer supports application side-loading.

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

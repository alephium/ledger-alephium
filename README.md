# Ledger Alephium Nano App

This project is still under active development.

## Build from source

The following command will build the releases for Ledger devices.

```shell
make release
```

## Install

You will need to install the tool `ledgerctl` to load Alphium app. The official guide is here: [https://github.com/LedgerHQ/ledgerctl#quick-install](https://github.com/LedgerHQ/ledgerctl#quick-install).

To install the app for Nano S:

```
make install_nanos
```

To install the app for Nano S+:

```
make install_nanosplus
```

Nano X manual installation isn't supported. The device no longer supports application side-loading.

## Uninstall

To uninstall the app:
```
ledgerctl delete Alephium
```

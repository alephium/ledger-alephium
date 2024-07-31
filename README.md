# Ledger Alephium App

## Build from source

To build the artifacts for Ledger devices, run the following command:

```shell
make release
```

## Test

### Test with Speculos

Start the Speculos simulator:

```shell
make run-speculos-<device>
```

Run the tests:

```shell
cd js/docker && docker compose up -d && cd ..
npm install && MODEL=<device> npm run speculos-test
```

### Test with a Ledger Device

Connect your Ledger device and run the tests:

```shell
cd js && npm run device-test
```

### Test a Single Test Case

To test a specific test case, change `it` to `it.only` in the test file `wallet.test.ts`. This allows Jest to run only that test case.

## Install

To install the Alephium app on your Ledger device, you will need the ledgerctl tool. Follow the official installation guide here: [https://github.com/LedgerHQ/ledgerctl#quick-install](https://github.com/LedgerHQ/ledgerctl#quick-install).

To install the app for Nano S:

```
make install_nanos
```

To install the app for Nano S+:

```
make install_nanosplus
```

Note: Manual installation for Nano X is not supported as the device no longer supports application side-loading.

## Uninstall

To uninstall the Alephium app from your Ledger device:
```
ledgerctl delete Alephium
```

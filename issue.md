After upgrading from ledger rust sdk `1.4.1` to `1.11.1`, we encountered display issues. Testing with speculos on nanos no problems, but there are display issues on nanosplus and nanox. Please refer to the screenshots in the `images` folder.

To reproduce:

1. `make release`
2. `make run-speculos-nanosplus`
3. `cd js/docker && docker compose up`
4. `cd js && npm install && npm run test -- test/speculos.test.ts`

Open `http://127.0.0.1:25000/` and test
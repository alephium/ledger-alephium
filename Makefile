app-builer-image:
	@docker build -t ledger-alephium-app-builder:latest -f ./configs/app-builder.Dockerfile configs

speculos-image:
	@docker build -t ledger-speculos:latest -f ./configs/speculos.Dockerfile configs

build:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest \
		bash -c " \
			cd app && \
			echo 'Building nanos app' && \
			cargo br --target=../configs/nanos.json && \
			echo 'Building nanosplus app' && \
			cargo br --target=../configs/nanosplus.json && \
			echo 'Building nanox app' && \
			cargo br --target=../configs/nanox.json \
		"

check:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest \
		bash -c " \
			cd app && \
			echo 'Cargo fmt' && \
			cargo fmt --all -- --check && \
			echo 'Cargo clippy' && \
			cargo clippy -Z build-std=core -Z build-std-features=compiler-builtins-mem --target=../configs/nanos.json \
		"

debug:
	@docker run --rm -it -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest

TARGET_HOST=https://raw.githubusercontent.com/LedgerHQ/ledger-nanos-sdk/master
update-configs:
	curl $(TARGET_HOST)/nanos.json --output configs/nanos.json
	curl $(TARGET_HOST)/nanosplus.json --output configs/nanosplus.json
	curl $(TARGET_HOST)/nanox.json --output configs/nanox.json

run-speculos:
	docker run --rm -it -v $(shell pwd)/app:/speculos/app \
		--publish 41000:41000 -p 5001:5000 -p 9999:9999 \
		ledger-speculos --display headless --vnc-port 41000 app/target/nanos/release/app

clean:
	cd app && cargo clean

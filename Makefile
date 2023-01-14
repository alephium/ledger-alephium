app-builer-image:
	@docker build -t ledger-alephium-app-builder:latest -f ./configs/app-builder.Dockerfile configs

speculos-image:
	@docker build -t ledger-speculos:latest -f ./configs/speculos.Dockerfile configs

install-nanos:
	@make _install device=nanos

install-nanox:
	@make _install device=nanox

install-nanosplus:
	@make _install device=nanosplus

_install:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest \
		bash -c " \
			cargo install --git https://github.com/LedgerHQ/cargo-ledger && \
			cd app && \
			echo 'Building nanos app' && \
			LEDGER_TARGETS=../configs/ RUST_BACKTRACE=1 cargo ledger -l $(device) -- --features device -Z unstable-options \
		"

build-release:
	@make _build type="--release" features="device"

build-debug:
	@make _build type="" features="debug"

_build:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest \
		bash -c " \
			cd app && \
			echo 'Building nanos app' && \
			cargo bembed $(type) --features $(features) --target=../configs/nanos.json && \
			echo 'Building nanosplus app' && \
			cargo bembed $(type) --features $(features) --target=../configs/nanosplus.json && \
			echo 'Building nanox app' && \
			cargo bembed $(type) --features $(features) --target=../configs/nanox.json \
		"

check:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest \
		bash -c " \
			cd app && \
			echo 'Cargo fmt' && \
			cargo fmt --all -- --check && \
			echo 'Cargo clippy' && \
			cargo clippy -Z build-std=core -Z build-std-features=compiler-builtins-mem --features=debug --target=../configs/nanos.json \
		"

debug:
	@docker run --rm -it -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ledger-alephium-app-builder:latest

TARGET_HOST=https://raw.githubusercontent.com/LedgerHQ/ledger-nanos-sdk/master
update-configs:
	curl $(TARGET_HOST)/nanos.json --output configs/nanos.json
	curl $(TARGET_HOST)/nanosplus.json --output configs/nanosplus.json
	curl $(TARGET_HOST)/nanox.json --output configs/nanox.json

# Webui: http://localhost:25000
run-speculos:
	docker run --rm -it -v $(shell pwd):/speculos/app \
		--publish 41000:41000 -p 25000:5000 -p 9999:9999 \
		ledger-speculos --display headless --vnc-port 41000 app/app/target/nanos/debug/app

clean:
	cd app && cargo clean

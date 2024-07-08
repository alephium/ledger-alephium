speculos-image:
	@docker build -t ledger-speculos:latest -f ./configs/speculos.Dockerfile configs

release:
	@make _release device=nanos
	@make _release device=nanox
	@make _release device=nanosplus

_release:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0 \
		bash -c " \
			cd app && \
			echo 'Building $(device) app' && \
			RUST_BACKTRACE=1 cargo ledger build $(device) -- -Z unstable-options && \
			cp ./target/$(device)/release/app.hex ../$(device).hex && \
			mv ./app_$(device).json ../$(device).json && \
			sed -i 's|target/$(device)/release/app.hex|$(device).hex|g;s|alph.gif|./app/alph.gif|g;s|alph_14x14.gif|./app/alph_14x14.gif|g' ../$(device).json \
		"

build-debug:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0 \
		bash -c " \
			cd app && \
			echo 'Building nanos app' && \
			cargo ledger build nanos -- --no-default-features --features debug && \
			echo 'Building nanosplus app' && \
			cargo ledger build nanosplus -- --no-default-features --features debug && \
			echo 'Building nanox app' && \
			cargo ledger build nanox -- --no-default-features --features debug \
		"

check:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0 \
		bash -c " \
			cd app && \
			echo 'Cargo fmt' && \
			cargo fmt --all -- --check && \
			echo 'Cargo clippy' && \
			cargo clippy -Z build-std=core -Z build-std-features=compiler-builtins-mem --target=nanos \
		"

debug:
	@docker run --rm -it -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0

# Webui: http://localhost:25000
run-speculos-nanos:
	docker run --rm -it -v $(shell pwd):/speculos/app \
		--publish 41000:41000 -p 25000:5000 -p 9999:9999 \
		ledger-speculos --model nanos --display headless --vnc-port 41000 app/app/target/nanos/release/app

run-speculos-nanosplus:
	docker run --rm -it -v $(shell pwd):/speculos/app \
		--publish 41000:41000 -p 25000:5000 -p 9999:9999 \
		ledger-speculos --model nanosp --display headless --vnc-port 41000 app/app/target/nanosplus/release/app

clean:
	cd app && cargo clean

set-github-action:
	make speculos-image
	make build-debug
	cd js/docker && docker compose up -d && cd ../..
	docker run -d --rm -v $(shell pwd):/speculos/app \
		--publish 41000:41000 -p 25000:5000 -p 9999:9999 \
		ledger-speculos --model nanos --display headless --vnc-port 41000 app/app/target/nanos/release/app

.PHONY: release clean

install_nanos:
	ledgerctl install -f nanos.json

install_nanosplus:
	ledgerctl install -f nanosplus.json

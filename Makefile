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

_run-speculos:
	docker run --rm -it -v $(shell pwd):/app --publish 5001:5001 --publish 9999:9999 -e DISPLAY='host.docker.internal:0' \
		-v '/tmp/.X11-unix:/tmp/.X11-unix' --privileged ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:3.27.0 \
		speculos -m $(device) /app/app/target/$(path)/release/app

run-speculos:
	@make run-speculos-nanos

run-speculos-nanos:
	@make _run-speculos device=nanos path=nanos

run-speculos-nanosplus:
	@make _run-speculos device=nanosp path=nanosplus

run-speculos-nanox:
	@make _run-speculos device=nanox path=nanox

clean:
	cd app && cargo clean

set-github-action:
	docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0
	docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:3.27.0
	make build-debug
	cd js/docker && docker compose up -d && cd ../..
	make run-speculos

.PHONY: release clean

install_nanos:
	ledgerctl install -f nanos.json

install_nanosplus:
	ledgerctl install -f nanosplus.json
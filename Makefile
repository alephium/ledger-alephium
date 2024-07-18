release:
	@make _release device=nanos
	@make _release device=nanox
	@make _release device=nanosplus
	@make _release device=stax
	@make _release device=flex

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

check:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0 \
		bash -c " \
			cd app && \
			echo 'Cargo fmt' && \
			cargo fmt --all -- --check && \
			echo 'Cargo clippy' && \
			cargo clippy -Z build-std=core -Z build-std-features=compiler-builtins-mem --target=nanos \
		"

_run-speculos:
	docker run --rm -it -v $(shell pwd):/app --publish 25000:5000 --publish 9999:9999 -e DISPLAY='host.docker.internal:0' \
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

run-speculos-stax:
	@make _run-speculos device=stax path=stax

run-speculos-flex:
	@make _run-speculos device=flex path=flex

clean:
	cd app && cargo clean

set-github-action:
	docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0
	docker pull ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:3.27.0
	cd js/docker && docker compose up -d && cd .. && npm ci && cd ..

run-github-ci:
	docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:3.27.0 \
		bash -c "cd app && cargo ledger build $(path) -- --no-default-features --features debug"
	docker run -d --name speculos --rm -v $(shell pwd):/app --publish 25000:5000 --publish 9999:9999 \
		ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:3.27.0 \
		speculos -m $(device) /app/app/target/$(path)/release/app --display headless
	cd js && sleep 3 && MODEL=$(device) npm run test && docker stop speculos && cd ..

.PHONY: release clean

install_nanos:
	ledgerctl install -f nanos.json

install_nanosplus:
	ledgerctl install -f nanosplus.json

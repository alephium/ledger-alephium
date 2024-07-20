version := 3.30.0
ledger_app_builder = ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder:$(version)
ledger_app_dev_tools = ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:$(version)

release:
	@make _release device=nanos
	@make _release device=nanox
	@make _release device=nanosplus
	@make _release device=stax
	@make _release device=flex

_release:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo $(ledger_app_builder) \
		bash -c " \
			cd app && \
			echo 'Building $(device) app' && \
			RUST_BACKTRACE=1 cargo ledger build $(device) -- -Z unstable-options && \
			cp ./target/$(device)/release/app.hex ../$(device).hex && \
			mv ./app_$(device).json ../$(device).json && \
			sed -i 's|target/$(device)/release/app.hex|$(device).hex|g;s|alph.gif|./app/alph.gif|g;s|alph_14x14.gif|./app/alph_14x14.gif|g' ../$(device).json \
		"

check:
	@docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo $(ledger_app_builder) \
		bash -c " \
			cd app && \
			echo 'Cargo fmt' && \
			cargo fmt --all -- --check && \
			echo 'Cargo clippy' && \
			cargo +nightly-2023-11-10 clippy --target=nanos && \
			cargo +nightly-2023-11-10 clippy --target=stax \
		"

_run-speculos:
	docker run --rm -it -v $(shell pwd):/app --publish 25000:5000 --publish 9999:9999 -e DISPLAY='host.docker.internal:0' \
		-v '/tmp/.X11-unix:/tmp/.X11-unix' --privileged $(ledger_app_dev_tools) \
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
	docker pull $(ledger_app_builder)
	docker pull $(ledger_app_dev_tools)
	cd js/docker && docker compose up -d && cd .. && npm ci && cd ..

run-github-ci:
	docker run --rm -v $(shell pwd):/app -v ledger-alephium-cargo:/opt/.cargo $(ledger_app_builder) \
		bash -c "cd app && cargo ledger build $(path) -- --no-default-features --features debug"
	docker run --name speculos --rm -v $(shell pwd):/app --publish 25000:5000 --publish 9999:9999 \
		$(ledger_app_dev_tools) speculos -m $(device) /app/app/target/$(path)/release/app --display headless &
	cd js && sleep 3 && MODEL=$(device) npm run speculos-test && docker stop speculos && cd ..

.PHONY: release clean

install_nanos:
	ledgerctl install -f nanos.json

install_nanosplus:
	ledgerctl install -f nanosplus.json

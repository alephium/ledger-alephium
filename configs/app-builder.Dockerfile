FROM ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder-lite:latest

# Define rustup/cargo home directories
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/.cargo

RUN apk add rustup curl

RUN rustup-init --default-toolchain nightly-2023-05-01 -y

# Adding cargo binaries to PATH
ENV PATH=${CARGO_HOME}/bin:${PATH}

# Adding the source code of the Rust standard library
RUN rustup component add rust-src rustfmt clippy

# Adding ARMV6M target to the default toolchain
RUN rustup target add thumbv6m-none-eabi
RUN cargo install --version 1.2.3 cargo-ledger
RUN cargo ledger setup

# Add a global Cargo config file (includes mandatory unstable features used to build our apps)
ADD ./cargo_global_config.toml $CARGO_HOME/config.toml

RUN apk update
RUN apk upgrade
RUN apk add python3-dev libusb musl-dev python3-dev libffi-dev openssl-dev pkgconfig curl

RUN pip3 install protobuf setuptools ecdsa
RUN pip3 install ledgerwallet

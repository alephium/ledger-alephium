FROM ghcr.io/ledgerhq/ledger-app-builder/ledger-app-builder-lite:latest

# Define rustup/cargo home directories
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/.cargo

RUN apk add rustup

RUN rustup-init --default-toolchain nightly -y

# Adding cargo binaries to PATH
ENV PATH=${CARGO_HOME}/bin:${PATH}

# Adding the source code of the Rust standard library
RUN rustup component add rust-src rustfmt clippy

# Adding ARMV6M target to the default toolchain
RUN rustup target add thumbv6m-none-eabi

RUN apk add python3-dev libusb musl-dev python3-dev libffi-dev openssl-dev pkgconfig
RUN pip3 install protobuf setuptools ecdsa
RUN pip3 install ledgerwallet

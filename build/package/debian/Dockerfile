ARG BASE_IMAGE=ubuntu:24.04
FROM ${BASE_IMAGE}

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:${PATH}

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libclang-dev \
    libdlib-dev \
    libopenblas-dev \
    liblapack-dev \
    libatlas-base-dev \
    libgtk-3-dev \
    libudev-dev \
    libpam0g-dev \
    debhelper \
    dpkg-dev \
    devscripts \
    curl \
    bzip2 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 --silent --show-error --fail https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal --no-modify-path

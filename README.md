# Botvana trading platform

Botvana is a open-source, event-driven, and distributed trading platform targetting
crypto markets. It aims to be high-performance, low-latency, reliable, and
fault-tolerant. Built in Rust, it allows you to build and operate market making
and arbitrage bots.

⚠️ The project is still in very early development ⚠️

## Principles:

-   High-performance: Designed and architected to be high-performance from the
    groud up.
-   Low-latency: Currently utilizing `io_uring` for network connectivity, with
    goals to use `AF_XDP` or kernel by-pass in the future.
-   Reliable: Being built in Rust provides Botvana with  memory safety guarantees
    compared to other systems programming languages.
-   Fault-tolerant: Crypto exchanges are known for unrealiability and so Botvana
    provides explicit fault-tolerant features.

## Overview

Botvana is split into these components:

-   `botnode`: trading bot
-   `botvana-server`: coordination server
-   `botvana`: shared platform definitions
-   `station-egui`: control application

### botnode

`botnode` is Botvana's trading bot. Botnode uses thread-per-core architecture where
each thread is pinned to exactly one logical CPU core. Each CPU core runs different
engine with custom event loop. Data is sent between engines using ring
channels and no state is shared between the threads.

Botnode has these engines:

- Control engine: connects to `botvana-server` and provides configuration for
  other engines
- Market data engine: connects to the exchange and transforms market data to
  Botvana's internal types.
- Indicator engine: provides indicators built from market data
- Trading engine: makes trading decisions
- Order engine: acts as order router and gateway
- Audit engine: audits trading activity

### botvana-server

Each `botnode` needs to connect to a central `botvana-server` which provides
configuration and acts as the central coordinator.

## Roadmap

- [x] Engine & channels implementation
- [ ] Reporting engine
- [ ] Infrastructure automation


## Development Prerequisites

In order to work with Botvana you need to have:

-   Linux kernel version >5.13
-   Rust 1.56 or higher
-   Terraform

## Getting started

1.  Clone the repo
    ```sh
    git clone https://github.com/featherenvy/botvana.git
    ```
2.  Install dependencies required to compile `egui`
    ```sh
    sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev
    ```
3.  Build all components
    ```sh
    cargo build
    ```

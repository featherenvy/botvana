# Botvana trading engine

Botvana is a distributed, event-driven, open-source trading system built using Rust.
The goal is to build a trading engine that is:

-   Fast by default: all choices made during design and development favor more
    performant solution where it's available.
-   Reliable: Botvana is built using Rust and leverages its ownership and type
    system, and concurency model.
-   Easy to operate: Ease of deployment, updating and operating the platform is
    a first class concern.

Note: The project is still in its infancy.

## Overview

Botvana is split into these components:

-   `botnode`: trading bots
-   `botvana-server`: coordination server
-   `botvana`: shared platform definitions
-   `station-egui`: control application

### botnode

`botnode` is Botvana's trading bot. Botnode uses thread-per-core architecture where
each thread is pinned to exactly one unique CPU core. Each CPU core runs different
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

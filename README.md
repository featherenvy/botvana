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

Botvana is built using these components:

-   `botnode`: trading bot
-   `Botvana-server`: coordination server
-   `botvana`: shared definitions
-   `station-egui`: control application

## Development Prerequisites

In order to work with Botvana you need to have installed:

-   Linux kernel version >5.13
-   Rust 1.56 or higher
-   Terraform

## Getting started

1.  Clone the repo
    ```sh
    git clone https://github.com/featherenvy/botvana.git
    ```
2.  Build all components
    ```sh
    cargo build
    ```

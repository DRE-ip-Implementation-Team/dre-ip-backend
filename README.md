# License
This work (except the DRE-ip protocol submodule) is licensed under the [AGPLv3 license](./LICENSE).
In short, you may use and modify it however you wish, as long as any unmodified or modified version you make available is also open-source under the same license.
Please refer to the license text for full conditions.

# Pre-built Verification Tools
The [GitHub releases page](https://github.com/DRE-ip-Implementation-Team/dre-ip-backend/releases) has pre-built verification tool binaries for 64-bit Windows and 64-bit Linux. The Linux binary was built on Ubuntu 22.04 LTS, but may work on other distributions.

# Building the Verification Tool
1. Ensure you have a [Rust toolchain](https://rustup.rs/) installed
2. Clone this repository
3. Ensure submodules are up-to-date (`git submodule update --init`) 
4. Run `cargo build --release --all-features --bin verification-cli`
5. The binary will be in `./target/release/`

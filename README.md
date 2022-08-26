# Pre-built Verification Tools
The [GitHub releases page](https://github.com/DRE-ip-Implementation-Team/dre-ip-backend/releases) has pre-built verification tool binaries for 64-bit Windows and 64-bit Linux. The Linux binary was built on Ubuntu 22.04 LTS, but may work on other distributions.

# Building the Verification Tool
1. Ensure you have a Rust toolchain installed (https://rustup.rs/)
2. Run `cargo build --release --all-features --bin verification-cli`
3. The binary will be in `./target/release/`

#!/usr/bin/env bash
cargo build --bin azure --release
cp ./target/release/azure ./azure/azure
pushd azure
zip -r ../azure.zip .
#!/usr/bin/env bash
set -xue

if ! [[ "$0" =~ scripts/example.run.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

# RUST_LOG=debug cargo run --example aws_cloudformation_ec2_instance_role.rs
cargo run --example aws_cloudformation_ec2_instance_role
cargo run --example aws_cloudformation_vpc
cargo run --example aws_ec2_key_pair
cargo run --example aws_kms
cargo run --example aws_s3
cargo run --example aws_sts

rm -f /tmp/test.insecure.key /tmp/test.insecure.cert
cargo run --example cert -- /tmp/test.insecure.key /tmp/test.insecure.cert
pushd ./avalanchego-compatibility
go run ./load-node-id/main.go /tmp/test.insecure.key /tmp/test.insecure.cert
popd

echo "ALL SUCCESS!"

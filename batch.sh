#!/bin/bash
set -ex

runs=100
workers="$(seq -s " " 50)"

cd "$(dirname "$0")"

# The packages need to be built in separate commands due to
# <https://github.com/rust-lang/cargo/issues/4463> or similar:
cargo build -r -p axum-hammer
cargo build -r -p axum-nail

cargo run -q -r -p axum-nail &
trap "kill $!" EXIT

while ! nc -w1 -z 127.0.0.1 8080
do sleep 1
done

now="$(date -u +%Y.%m.%d-%H.%M.%SZ)"
mkdir -p stats/one-service-"$now"

while read -r urlpath filename
do
    cargo run -q -r -p axum-hammer -- run \
        -o "stats/one-service-$now/$filename.json" \
        "http://127.0.0.1:8080/$urlpath" \
        "$runs" \
        $workers
done <<EOT
hello hello
hello-service hello-service
EOT

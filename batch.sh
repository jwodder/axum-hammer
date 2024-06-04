#!/bin/bash
set -ex

runs=100
samples=25
#workers='5 10 15 20 21 22 23 24 25 26 27 28 29 30 40 50 100'
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
mkdir -p stats/"$now"

while read -r urlpath filename
do
    cargo run -q -r -p axum-hammer -- run \
        -o "stats/$now/$filename.json" \
        "http://127.0.0.1:8080/$urlpath" \
        "$runs" \
        $workers
done <<EOT
hello hello
sleep?min=10&max=25 sleep-10-25
foo foo
foo/bar foo-bar
foo/custom foo-custom
simple-service simple-service
EOT

for endpoint in subpages subpages-arc subpages-service subpages-service-arc
do
    cargo run -q -r -p axum-hammer -- subpages \
        -o "stats/$now/$endpoint.json" \
        --samples "$samples" \
        "http://127.0.0.1:8080/$endpoint" \
        $workers
done

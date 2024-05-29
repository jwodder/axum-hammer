#!/bin/bash
set -ex

cd "$(dirname "$0")"

now="$(date -u +%Y.%m.%d-%H.%M.%SZ)"

mkdir -p stats/"$now"

cargo build -r -p axum-hammer -p axum-nail

cargo run -r -p axum-nail &
trap "kill $!" EXIT

while ! nc -w1 -z 127.0.0.1 8080
do sleep 1
done

for endpoint in subpages subpages-arc subpages-service subpages-service-arc
do
    cargo run -q -r -p axum-hammer -- \
        subpages --samples 25 "http://127.0.0.1:8080/$endpoint" \
        5 10 15 20 21 22 23 24 25 26 27 28 29 30 40 50 100 \
        > "stats/$now/$endpoint.csv"
done

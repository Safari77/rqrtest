#!/bin/bash
export NOW=$(date +%s)||exit 1
export DUMPOUT=qrtests-"${NOW}".txt

if [[ ! -x ./target/release/rqrtest ]]; then
    echo ./target/release/rqrtest must be executable
    exit 1
fi

for ecc in M H; do
    (echo ECC level $ecc; for i in $(seq 1 32); do
        if [[ -f rmqr_versions/rmqr_v"${i}"_"${ecc}".png ]]; then
            export CORRECT=$(cat rmqr_versions/rmqr_v"${i}"_"${ecc}".txt)
            ./target/release/rqrtest --rmqr rmqr_versions/rmqr_v"${i}"_"${ecc}".png >& output-rmqr_v"${i}"_"${ecc}".txt
            pcre2grep "^Found rMQR.*: ${CORRECT}$" output-rmqr_v"${i}"_"${ecc}".txt
        fi
    done; echo) 2>&1 >> "${DUMPOUT}"
done
echo "${DUMPOUT}"
cat "${DUMPOUT}"

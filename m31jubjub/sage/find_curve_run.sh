#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <nthreads>"
    exit 1
fi

nthreads=$1

for ((i=1; i<=nthreads; i++)); do
    sage find_curve.sage $i $nthreads &
done

wait

echo "All processes completed"
#!/bin/sh
# sim (default): small Monte Carlo run, prints the false-pass rate. Labelled SIMULATION.
set -e
case "${1:-sim}" in
  sim)
    echo "SIMULATION: scoring engine under assumed generators (not a model result)"
    montecarlo --n 20000 --seeds 3 --boot 200 --auc-boot 20 --out /tmp/mc.json
    ;;
  *) exec "$@" ;;
esac

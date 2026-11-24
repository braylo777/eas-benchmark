# EAW benchmark, simulation path (Study S). No model, no network, no GPU.
# build: docker build -t eaw-benchmark .
# run:   docker run --rm eaw-benchmark            (small Monte Carlo smoke run, a few seconds)
#        docker run --rm eaw-benchmark test       (all Rust tests)
FROM rust:1-slim AS build
WORKDIR /src
COPY rust/ rust/
WORKDIR /src/rust
RUN cargo build --release --bins --locked

FROM debian:stable-slim
RUN useradd --create-home eaw
WORKDIR /home/eaw
COPY --from=build /src/rust/target/release/montecarlo /usr/local/bin/montecarlo
COPY --from=build /src/rust/target/release/pressure /usr/local/bin/pressure
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
USER eaw
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["sim"]

from rust:latest

RUN cargo install cargo-shuttle
RUN cargo shuttle run --release

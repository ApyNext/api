from rust:latest

WORKDIR /app

COPY . .

RUN cargo install cargo-shuttle
CMD ["cargo", "shuttle", "run", "--release"]

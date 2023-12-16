from rust:latest
ARG FRONT_URL

WORKDIR /app

COPY . .

RUN cargo build --release
CMD ["cargo", "run", "--release"]

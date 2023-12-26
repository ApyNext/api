from rust:latest
ARG FRONT_URL
ARG EMAIL_NAME
ARG EMAIL_CONFIRM_ROUTE
ARG A2F_ROUTE

WORKDIR /app

COPY . .

RUN cargo build --release
CMD ["cargo", "run", "--release"]

# Build the current checkout so the image always contains this project's code.
FROM rust:1.85-bookworm AS builder

WORKDIR /src

# Keep dependency compilation cacheable when only application sources change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked --release \
    && rm -rf src

COPY src ./src
RUN cargo build --locked --release

# The runtime image only needs the compiled executable and libc.
FROM debian:bookworm-slim

COPY --from=builder /src/target/release/rtop /usr/local/bin/rtop

ENV TERM=xterm-256color

ENTRYPOINT ["rtop"]

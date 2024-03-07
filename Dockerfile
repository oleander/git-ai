# Use the official Rust image as a builder stage
FROM rust:latest as builder

# Create a new empty shell project
RUN USER=root cargo new --bin git-ai
WORKDIR /git-ai

# Copy your Rust project's files into the Docker image
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Cache your dependencies
RUN mkdir src/bin
RUN touch src/bin/hook.rs
RUN touch src/main.rs
RUN touch src/lib.rs
RUN cargo fetch

# Copy the rest of your code
COPY . .

# Build your application
RUN cargo build --bins
# RUN cargo install --debug --path .

# Final base image
FROM debian:buster-slim

# Copy the binary from the builder stage to the final stage
COPY --from=builder /git-ai/target/debug/git-ai /usr/local/bin/git-ai
COPY --from=builder /git-ai/target/debug/git-ai-hook /usr/local/bin/git-ai-hook

# Install git
RUN apt-get update && apt-get install -y git && apt-get clean

# Set the working directory
WORKDIR /repo

# By default, run your application
CMD ["git-ai"]

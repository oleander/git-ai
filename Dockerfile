# Use a specific Rust version
FROM rust:1.76-slim

WORKDIR /git-ai

# Copy project files and build the project
COPY . .
RUN rustc -vV && cargo test
RUN cargo build --bin git-ai-hook --release

# Use a slim version of Debian for the final image
# FROM debian:buster-slim
COPY /git-ai/target/release/git-ai /usr/local/bin/git-ai
COPY /git-ai/target/release/git-ai-hook /usr/local/bin/git-ai-hook

# Install required packages and Rust
# RUN apt-get update && \
#     apt-get install -y curl build-essential git && \
#     # curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
#     apt-get clean && \
#     rm -rf /var/lib/apt/lists/*

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Add a non-root user and switch to it
RUN useradd -m ai-bot
USER ai-bot

WORKDIR /repo
SHELL ["/bin/bash", "-c"]
CMD ["git-ai"]

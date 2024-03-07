# Use a specific Rust version
FROM rust:1.76.0 as builder
WORKDIR /git-ai

# Copy project files and build the project
COPY . .
RUN cargo build --bins

# Use a slim version of Debian for the final image
FROM debian:buster-slim
COPY --from=builder /git-ai/target/debug/git-ai /usr/local/bin/git-ai
COPY --from=builder /git-ai/target/debug/git-ai-hook /usr/local/bin/git-ai-hook

# Install git and clean up in one layer
RUN apt-get update && \
    apt-get install -y git && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Add a non-root user and switch to it
RUN useradd -m ai-bot
USER ai-bot

WORKDIR /repo

CMD ["git-ai"]

# Use an official Rust image as a parent image
FROM rust:latest AS base

# Install dependencies
RUN apt-get update && apt-get install -y git fish

# Set the working directory inside the container
WORKDIR /app

# Copy the current directory contents into the container at /usr/src/myapp
COPY . .

# Install the git-ai from the source
RUN cargo install --debug --path .

# Make sure .env file exists (for demonstration, you might want to handle this differently)
COPY ./scripts/integration-tests scripts/integration-tests

# Run the script
CMD ["fish", "./scripts/integration-tests"]

# Target for testing PRs with GH CLI
FROM rust:latest AS pr-tester

# Install wget and GH CLI
RUN apt-get update && apt-get install -y git fish wget \
    && mkdir -p -m 755 /etc/apt/keyrings \
    && wget -nv -O /tmp/githubcli-archive-keyring.gpg https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    && cat /tmp/githubcli-archive-keyring.gpg | tee /etc/apt/keyrings/githubcli-archive-keyring.gpg > /dev/null \
    && chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/* \
    && git config --global user.email "test@example.com" \
    && git config --global user.name "Test User"

RUN rustup default nightly
RUN rustup component add rust-std clippy rustc rustfmt --toolchain nightly

WORKDIR /app

ARG GH_TOKEN
ENV GH_TOKEN=$GH_TOKEN
RUN gh repo clone oleander/git-ai /app
RUN git remote set-url origin https://x-access-token:$GH_TOKEN@github.com/oleander/git-ai.git
RUN cargo fetch
RUN cargo build
RUN cargo clippy

ARG PR_NUMBER
RUN gh pr checkout $PR_NUMBER
RUN cargo fetch
RUN cargo build
RUN cargo clippy

# Default command that can be overridden
SHELL ["/bin/bash", "-lc"]
CMD ["bash"]

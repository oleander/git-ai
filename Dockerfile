# Use an official Rust image as a parent image
FROM rust:latest

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

# Use a Rust base image
FROM rust:latest as build

# Set the working directory
WORKDIR /app

# Copy the Rocket server code to the container
COPY . .

# Build the server
RUN cargo build --release

# Set up the runtime environment
FROM debian:bullseye-slim
WORKDIR /app

# Install OpenSSL and other dependencies
RUN apt-get update && \
    apt-get install -y openssl ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the built server from the previous stage
COPY --from=build /app/target/release/server .
COPY ssl/production/fantasypvp.uk.pem ./ssl/production/fantasypvp.uk.pem
COPY ssl/production/fantasypvp.uk.key ./ssl/production/fantasypvp.uk.key
COPY Rocket.toml ./
# Expose the desired port
EXPOSE 8000

# Run the server
CMD ["./server"]

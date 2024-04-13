FROM rust:1.76-slim-buster as builder
WORKDIR /app
# Explicitly copy source and data 
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/ 
COPY data/ ./data/
RUN ls -la /app

RUN cargo install --path .

FROM debian:buster-slim as runner
# Copy the binary to the production image from the builder stage.
COPY --from=builder /usr/local/cargo/bin/geojson-api /usr/local/bin/geojson-api
# Copy the data to the production image
COPY --from=builder /app/data/ /app/data/

ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8080
CMD ["sh", "-c", "cd /app && geojson-api"]
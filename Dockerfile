####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev
RUN update-ca-certificates

# Create appuser
ENV USER=faraway
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /faraway

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release
# RUN strip -s /faraway/target/release/faraway

####################################################################################################
## Final image
####################################################################################################
FROM alpine

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

EXPOSE 9000

WORKDIR /faraway

# Copy our build
COPY --from=builder /faraway/target/x86_64-unknown-linux-musl/release/faraway ./

# Use an unprivileged user.
USER faraway:faraway

CMD ["/faraway/faraway"]
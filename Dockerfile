# Specify the parent image from which we build
FROM debian:bookworm-slim
RUN apt-get update
RUN apt-get update && apt-get install -y \
    gcc-arm-linux-gnueabihf \
    curl
    # \
    # build-essential \

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
ENV PATH="$PATH:/root/.cargo/bin"
RUN rustup target add armv7-unknown-linux-gnueabihf

# Execute command
USER root
WORKDIR /container/project
CMD ["/bin/bash", "build-container.sh"]
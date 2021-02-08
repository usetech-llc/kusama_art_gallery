# ===== BUILD ======

FROM phusion/baseimage:0.10.2 as builder
LABEL maintainer="gz@usetech.com"

ENV WASM_TOOLCHAIN=nightly-2021-01-08

ARG PROFILE=release

RUN apt-get update && \
	apt-get dist-upgrade -y -o Dpkg::Options::="--force-confold" && \
	apt-get install -y cmake pkg-config libssl-dev git clang

# Get project and run it
#RUN git clone https://github.com/usetech-llc/kusama_gallery /kusama_gallery
RUN mkdir kusama_gallery
WORKDIR /kusama_gallery
COPY . .

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN	rustup toolchain install $WASM_TOOLCHAIN
RUN	rustup target add wasm32-unknown-unknown --toolchain $WASM_TOOLCHAIN
RUN rustup target list --installed
RUN rustup default nightly-2021-01-08-x86_64-unknown-linux-gnu
RUN rustup show
RUN	cargo build "--$PROFILE" 
	# && \
	# cargo test

RUN cd target/release && ls -la

# ===== RUN ======

FROM phusion/baseimage:0.10.2
ARG PROFILE=release

COPY --from=builder /kusama_gallery/target/$PROFILE/node-template /usr/local/bin

EXPOSE 9944
VOLUME ["/chain-data"]

# Copy and run start script
COPY ["./run.sh", "./run.sh"]
RUN chmod +x ./run.sh
CMD ["bash", "-c", "./run.sh"]
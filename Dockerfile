FROM rust:1.34

WORKDIR /src
COPY . .

CMD cargo test --verbose
FROM rust:latest as builder

WORKDIR /usr/src/app

# Instale as dependências necessárias
RUN apt-get update && apt-get install -y \
    musl-tools \
    gcc \
    libssl-dev \
    pkg-config \
    make

# Configure o compilador Rust para usar o musl
RUN rustup target add x86_64-unknown-linux-musl

# Copie os arquivos do projeto para o container
COPY . .

# Compile a aplicação com o alvo musl (estático)
RUN cargo build --release --target=x86_64-unknown-linux-musl

# Etapa final para criar a imagem leve
FROM debian:buster-slim

WORKDIR /app

# Copie o binário compilado para a nova imagem
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/rustle_chat .

# Porta exposta
EXPOSE 3000

# Comando para rodar a aplicação
CMD ["./rustle_chat"]


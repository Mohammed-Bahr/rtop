FROM debian:bookworm-slim

ARG INSTALL_URL=https://raw.githubusercontent.com/Mohammed-Bahr/rtop/main/install.sh

ENV PATH="/root/.local/bin:${PATH}"

RUN apt-get update \
    && apt-get install --no-install-recommends --yes \
        ca-certificates \
        curl \
        tar \
        unzip \
    && rm -rf /var/lib/apt/lists/* \
    && curl -fsSL "$INSTALL_URL" | sh

CMD ["rtop", "--version"]

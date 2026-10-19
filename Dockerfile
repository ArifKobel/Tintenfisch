# syntax=docker/dockerfile:1

FROM node:24-bookworm-slim AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --target wasm32-unknown-unknown
ENV PATH=/root/.cargo/bin:$PATH
RUN corepack enable

WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    corepack install && pnpm install --frozen-lockfile

COPY . .
ARG VITE_SITE_URL
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/app/engine/target \
    pnpm build

FROM nginx:1.29-alpine
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /usr/share/nginx/html
EXPOSE 80

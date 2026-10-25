FROM elixir:1.19-otp-28 AS build
ENV MIX_ENV=prod \
    MIX_REBAR3=/usr/local/bin/rebar3 \
    HEX_HTTP_TIMEOUT=180 \
    HEX_HTTP_CONCURRENCY=2 \
    HEX_CACERTS_PATH=/etc/ssl/certs/ca-certificates.crt
ARG HEX_SOURCE_URL=https://github.com/hexpm/hex/archive/refs/tags/v2.6.0.tar.gz
ARG REBAR3_ARCHIVE_URL=https://github.com/erlang/rebar3/releases/download/3.25.1/rebar3
WORKDIR /app
COPY docker/ /usr/local/lib/gastro-build/
RUN --mount=type=cache,id=gastro-gateway-tools,target=/root/.cache/gastro-build-tools,sharing=locked \
    sh /usr/local/lib/gastro-build/bootstrap-tools.sh
COPY mix.exs mix.lock ./
RUN --mount=type=cache,id=gastro-gateway-hex,target=/root/.hex,sharing=locked \
    sh /usr/local/lib/gastro-build/retry.sh mix deps.get --only prod && mix deps.compile
COPY lib ./lib
RUN mix compile --warnings-as-errors && mix release
FROM elixir:1.19-otp-28
RUN useradd --system --uid 10001 gastro
WORKDIR /app
COPY --from=build --chown=gastro /app/_build/prod/rel/gastro_gateway ./
USER gastro
ENV PORT=4000
EXPOSE 4000
CMD ["bin/gastro_gateway", "start"]

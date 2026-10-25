FROM elixir:1.19-otp-28 AS build
ENV MIX_ENV=prod
WORKDIR /app
RUN mix local.hex --force && mix local.rebar --force
COPY mix.exs mix.lock ./
RUN mix deps.get --only prod && mix deps.compile
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

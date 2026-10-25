defmodule Gastro.MixProject do
  use Mix.Project

  def project,
    do: [
      app: :gastro_gateway,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]

  def application, do: [extra_applications: [:logger, :ssl], mod: {Gastro.Application, []}]

  defp deps do
    [
      {:bandit, "~> 1.6"},
      {:websock_adapter, "~> 0.5"},
      {:plug, "~> 1.16"},
      {:req, "~> 0.5"},
      {:jason, "~> 1.4"}
    ]
  end
end

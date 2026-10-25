defmodule Gastro.Application do
  use Application

  def start(_type, _args) do
    port = String.to_integer(System.get_env("PORT", "4000"))

    Supervisor.start_link(
      [
        {Task.Supervisor, name: Gastro.Tasks},
        {Bandit, plug: Gastro.Router, port: port, ip: {0, 0, 0, 0}}
      ],
      strategy: :one_for_one,
      name: Gastro.Supervisor
    )
  end
end

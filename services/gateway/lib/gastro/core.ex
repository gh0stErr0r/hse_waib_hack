defmodule Gastro.Core do
  def request(method, path, token, body \\ nil) do
    opts = [
      method: method,
      url: System.get_env("CORE_URL", "http://engine:8082") <> path,
      headers: [{"authorization", "Bearer " <> token}],
      receive_timeout: 250_000,
      connect_options: [timeout: 5_000],
      retry: false,
      redirect: false
    ]

    opts = if body, do: Keyword.put(opts, :json, body), else: opts

    case Req.request(opts) do
      {:ok, %{status: status, body: response}} when is_map(response) or is_list(response) ->
        {status, response}

      _ ->
        {502, %{"type" => "error", "text" => "Ядро временно недоступно"}}
    end
  end
end

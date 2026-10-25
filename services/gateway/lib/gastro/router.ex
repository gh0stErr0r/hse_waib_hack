defmodule Gastro.Router do
  use Plug.Router
  plug(:match)
  plug(Plug.Parsers, parsers: [:json], json_decoder: Jason, length: 2_097_152)
  plug(:dispatch)

  get "/health" do
    send_resp(conn, 200, "ok")
  end

  get "/ws" do
    origins =
      System.get_env("ALLOWED_ORIGINS", "http://localhost:8088") |> String.split(",", trim: true)

    if get_req_header(conn, "origin") in Enum.map(origins, &[&1]) do
      WebSockAdapter.upgrade(conn, Gastro.Socket, %{}, timeout: 300_000, max_frame_size: 16_384)
    else
      send_resp(conn, 403, "Origin denied")
    end
  end

  match _ do
    path = conn.request_path

    allowed =
      Regex.match?(
        ~r"\A/api/v1/(conversations(?:/[0-9a-f-]{36}(?:/messages)?)?|rank|session)\z",
        path
      )

    token =
      case get_req_header(conn, "authorization") do
        ["Bearer " <> token] when byte_size(token) > 0 and byte_size(token) < 8192 -> token
        _ -> nil
      end

    cond do
      !allowed ->
        send_resp(conn, 404, "Not found")

      !token ->
        send_resp(conn, 401, "Unauthorized")

      conn.method not in ["GET", "POST"] ->
        send_resp(conn, 405, "Method not allowed")

      true ->
        query = if conn.query_string == "", do: "", else: "?" <> conn.query_string
        method = if conn.method == "GET", do: :get, else: :post
        body = if method == :post, do: conn.body_params, else: nil
        {status, response} = Gastro.Core.request(method, path <> query, token, body)

        conn
        |> put_resp_content_type("application/json")
        |> send_resp(status, Jason.encode!(response))
    end
  end
end

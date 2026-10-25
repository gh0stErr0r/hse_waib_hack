defmodule Gastro.MockCore do
  use Plug.Router
  plug(:match)
  plug(Plug.Parsers, parsers: [:json], json_decoder: Jason)
  plug(:dispatch)

  match _ do
    {status, payload} =
      if Plug.Conn.get_req_header(conn, "authorization") == ["Bearer accepted"] do
        {200,
         %{
           type: "assistant",
           text: "Что предпочитаете?",
           dynamic_buttons: [%{label: "Рамен", value: "Рамен"}],
           slots: []
         }}
      else
        {401, %{type: "error", text: "Invalid token"}}
      end

    conn |> put_resp_content_type("application/json") |> send_resp(status, Jason.encode!(payload))
  end
end

defmodule Gastro.SessionTest do
  use ExUnit.Case, async: false

  setup_all do
    server = start_supervised!({Bandit, plug: Gastro.MockCore, port: 0, ip: {127, 0, 0, 1}})
    {:ok, {_address, port}} = ThousandIsland.listener_info(server)
    previous = System.get_env("CORE_URL")
    System.put_env("CORE_URL", "http://127.0.0.1:#{port}")

    on_exit(fn ->
      if previous, do: System.put_env("CORE_URL", previous), else: System.delete_env("CORE_URL")
    end)

    :ok
  end

  test "Bearer auth, supervised work, dynamic buttons and backpressure" do
    {:ok, initial} = Gastro.Socket.init(%{})

    {:push, {:text, ready}, state} =
      Gastro.Socket.handle_in({~s({"type":"auth","token":"accepted"}), [opcode: :text]}, initial)

    assert Jason.decode!(ready)["type"] == "ready"

    request =
      Jason.encode!(%{
        type: "message",
        conversation_id: "11111111-1111-4111-8111-111111111111",
        request_id: "22222222-2222-4222-8222-222222222222",
        text: "Рамен"
      })

    {:push, {:text, _}, busy} = Gastro.Socket.handle_in({request, [opcode: :text]}, state)
    {:push, {:text, rejected}, _} = Gastro.Socket.handle_in({request, [opcode: :text]}, busy)
    assert Jason.decode!(rejected)["status"] == 429
    assert_receive {ref, {200, _}} = result, 5000
    assert ref == busy.task.ref
    {:push, {:text, response}, complete} = Gastro.Socket.handle_info(result, busy)

    assert Jason.decode!(response)["dynamic_buttons"] == [
             %{"label" => "Рамен", "value" => "Рамен"}
           ]

    refute complete.busy
  end

  test "invalid auth closes the socket" do
    {:ok, state} = Gastro.Socket.init(%{})

    assert {:stop, :normal, 4401, _} =
             Gastro.Socket.handle_in(
               {~s({"type":"auth","token":"forged"}), [opcode: :text]},
               state
             )

    Process.cancel_timer(state.timer)
  end

  test "cross-origin websocket upgrade is refused" do
    conn =
      Plug.Test.conn(:get, "/ws")
      |> Plug.Conn.put_req_header("origin", "https://unexpected.example")

    assert Gastro.Router.call(conn, Gastro.Router.init([])).status == 403
  end

  test "REST proxy rejects unauthorized and unsupported paths" do
    assert Gastro.Router.call(Plug.Test.conn(:get, "/api/v1/conversations"), []).status == 401

    conn =
      Plug.Test.conn(:get, "/arbitrary-url")
      |> Plug.Conn.put_req_header("authorization", "Bearer accepted")

    assert Gastro.Router.call(conn, []).status == 404
  end
end

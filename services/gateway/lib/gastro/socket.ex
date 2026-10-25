defmodule Gastro.Socket do
  @behaviour WebSock
  def init(_) do
    timer = Process.send_after(self(), :auth_timeout, 10_000)
    {:ok, %{token: nil, timer: timer, busy: false, task: nil, request_id: nil}}
  end

  def handle_in({raw, [opcode: :text]}, state) when byte_size(raw) <= 16_384 do
    case Jason.decode(raw) do
      {:ok, message} when is_map(message) -> dispatch(message, state)
      _ -> reply(%{type: "error", text: "Некорректный JSON"}, state)
    end
  end

  def handle_in(_, state), do: reply(%{type: "error", text: "Ожидается текстовый JSON"}, state)

  defp dispatch(%{"type" => "auth", "token" => token}, %{token: nil} = state)
       when is_binary(token) and byte_size(token) > 0 and byte_size(token) < 8192 do
    case Gastro.Core.request(:get, "/api/v1/session", token) do
      {200, _} ->
        Process.cancel_timer(state.timer)
        reply(%{type: "ready"}, %{state | token: token})

      _ ->
        {:stop, :normal, 4401, state}
    end
  end

  defp dispatch(_, %{token: nil} = state),
    do: reply(%{type: "error", text: "Сначала авторизуйтесь", status: 401}, state)

  defp dispatch(%{"type" => "ping"}, state), do: reply(%{type: "pong"}, state)

  defp dispatch(_, %{busy: true} = state),
    do: reply(%{type: "error", text: "Дождитесь ответа на предыдущий запрос", status: 429}, state)

  defp dispatch(
         %{
           "type" => "message",
           "conversation_id" => id,
           "request_id" => request_id,
           "text" => text
         },
         state
       )
       when is_binary(id) and is_binary(request_id) and is_binary(text) do
    valid =
      Regex.match?(~r/\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/, id) &&
        Regex.match?(
          ~r/\A[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\z/,
          request_id
        ) &&
        String.length(String.trim(text)) in 1..4000

    if valid do
      token = state.token

      task =
        Task.Supervisor.async_nolink(Gastro.Tasks, fn ->
          Gastro.Core.request(:post, "/api/v1/conversations/#{id}/messages", token, %{
            request_id: request_id,
            text: text
          })
        end)

      reply(%{type: "thinking", request_id: request_id}, %{
        state
        | busy: true,
          task: task,
          request_id: request_id
      })
    else
      reply(
        %{type: "error", text: "Некорректный идентификатор или текст", request_id: request_id},
        state
      )
    end
  end

  defp dispatch(_, state), do: reply(%{type: "error", text: "Неизвестный тип сообщения"}, state)

  def handle_info(:auth_timeout, %{token: nil} = state), do: {:stop, :normal, 4401, state}
  def handle_info(:auth_timeout, state), do: {:ok, state}

  def handle_info({ref, {status, body}}, %{task: %{ref: ref}} = state) do
    Process.demonitor(ref, [:flush])

    response =
      if status in 200..299,
        do: body,
        else:
          Map.merge(body, %{
            "type" => "error",
            "status" => status,
            "request_id" => state.request_id
          })

    reply(response, %{state | busy: false, task: nil, request_id: nil})
  end

  def handle_info({:DOWN, ref, _, _, _}, %{task: %{ref: ref}} = state) do
    reply(%{type: "error", text: "Не удалось обработать запрос", request_id: state.request_id}, %{
      state
      | busy: false,
        task: nil,
        request_id: nil
    })
  end

  def handle_info(_, state), do: {:ok, state}
  def terminate(_, %{task: %{pid: pid}}), do: Task.Supervisor.terminate_child(Gastro.Tasks, pid)
  def terminate(_, _), do: :ok
  defp reply(body, state), do: {:push, {:text, Jason.encode!(body)}, state}
end

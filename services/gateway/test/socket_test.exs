defmodule Gastro.SocketTest do
  use ExUnit.Case

  test "unauthenticated clients cannot submit work" do
    {:ok, state} = Gastro.Socket.init(%{})

    {:push, {:text, payload}, ^state} =
      Gastro.Socket.handle_in({~s({"type":"message","text":"hi"}), [opcode: :text]}, state)

    assert Jason.decode!(payload)["status"] == 401
    Process.cancel_timer(state.timer)
  end

  test "invalid JSON leaves the session usable" do
    {:ok, state} = Gastro.Socket.init(%{})
    assert {:push, {:text, _}, ^state} = Gastro.Socket.handle_in({"{", [opcode: :text]}, state)
    Process.cancel_timer(state.timer)
  end
end

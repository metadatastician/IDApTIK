defmodule IdaptikServerWeb.UserSocket do
  use Phoenix.Socket

  # Every game session is a topic under "session:". The two roles join the same
  # session id from opposite ends.
  channel "session:*", IdaptikServerWeb.SessionChannel

  @impl true
  def connect(_params, socket, _connect_info) do
    {:ok, socket}
  end

  # Anonymous for now; identity/matchmaking is a later milestone.
  @impl true
  def id(_socket), do: nil
end

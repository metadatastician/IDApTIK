defmodule IdaptikServerWeb.StatusPlug do
  @moduledoc """
  Terminal plug for the endpoint. The game talks to this server over Phoenix
  Channels (WebSocket); there is no HTML/API surface, so plain HTTP requests get
  a small JSON status instead of falling through to a 500.
  """

  import Plug.Conn

  def init(opts), do: opts

  def call(conn, _opts) do
    body =
      Jason.encode!(%{
        service: "idaptik-server",
        transport: "phoenix-channels-over-bandit",
        status: "ok"
      })

    conn
    |> put_resp_content_type("application/json")
    |> send_resp(200, body)
    |> halt()
  end
end

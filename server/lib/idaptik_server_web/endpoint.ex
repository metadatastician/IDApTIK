defmodule IdaptikServerWeb.Endpoint do
  use Phoenix.Endpoint, otp_app: :idaptik_server

  # The realtime surface: one socket carrying the game sessions (ADR-0002).
  socket "/socket", IdaptikServerWeb.UserSocket,
    websocket: true,
    longpoll: false

  plug Plug.RequestId
  plug Plug.Telemetry, event_prefix: [:phoenix, :endpoint]

  # This backend is sockets, not an HTTP API — a single terminal plug answers
  # plain HTTP with a small status so requests get a clean response.
  plug IdaptikServerWeb.StatusPlug
end

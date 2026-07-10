defmodule IdaptikServer.Application do
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      {Phoenix.PubSub, name: IdaptikServer.PubSub},
      IdaptikServerWeb.Endpoint
    ]

    opts = [strategy: :one_for_one, name: IdaptikServer.Supervisor]
    Supervisor.start_link(children, opts)
  end

  @impl true
  def config_change(changed, _new, removed) do
    IdaptikServerWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end

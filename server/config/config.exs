import Config

# Serve the endpoint with Bandit (ADR-0002).
config :idaptik_server, IdaptikServerWeb.Endpoint,
  adapter: Bandit.PhoenixAdapter,
  pubsub_server: IdaptikServer.PubSub

config :phoenix, :json_library, Jason

import_config "#{config_env()}.exs"

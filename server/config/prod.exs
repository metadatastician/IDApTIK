import Config

# The native Bevy clients connect directly to Phoenix Channels and may use a
# LAN address, tailnet address, or forwarded public address. There is no
# browser-origin trust boundary on this socket; role/session validation remains
# in the channel layer. Bind address, port, and secrets are supplied at runtime
# by runtime.exs so no deployment credentials enter the release artifact.
config :idaptik_server, IdaptikServerWeb.Endpoint, check_origin: false

config :logger, level: :info
config :phoenix, :plug_init_mode, :runtime

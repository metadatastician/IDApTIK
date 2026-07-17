import Config

config :idaptik_server, IdaptikServerWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4002],
  # Test-only placeholder (>= 64 bytes), mirroring dev.exs; prod reads
  # SECRET_KEY_BASE from the environment in runtime.exs.
  secret_key_base: "test_only_not_a_secret_change_me_000000000000000000000000000000000000000",
  # Channel tests talk to the endpoint in-process; no listener needed.
  server: false

config :logger, level: :warning
config :phoenix, :plug_init_mode, :runtime

import Config

config :idaptik_server, IdaptikServerWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4000],
  check_origin: false,
  debug_errors: true,
  # Dev-only placeholder (>= 64 bytes). Prod reads SECRET_KEY_BASE from the
  # environment in runtime.exs — never commit a real secret here.
  secret_key_base: "dev_only_not_a_secret_change_me_0000000000000000000000000000000000000000",
  watchers: []

config :logger, :console, format: "[$level] $message\n"
config :phoenix, :stacktrace_depth, 20
config :phoenix, :plug_init_mode, :runtime

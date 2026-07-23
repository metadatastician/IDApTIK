import Config

config :idaptik_server, IdaptikServerWeb.Endpoint,
  # IDAPTIK_PORT lets the loopback gate (scripts/loopback_check.sh) run a
  # throwaway relay without colliding with a dev server on 4000.
  http: [ip: {127, 0, 0, 1}, port: String.to_integer(System.get_env("IDAPTIK_PORT") || "4000")],
  check_origin: false,
  debug_errors: true,
  # Dev-only placeholder (>= 64 bytes). Prod reads SECRET_KEY_BASE from the
  # environment in runtime.exs — never commit a real secret here.
  secret_key_base: "dev_only_not_a_secret_change_me_0000000000000000000000000000000000000000",
  watchers: []

config :logger, :console, format: "[$level] $message\n"
config :phoenix, :stacktrace_depth, 20
config :phoenix, :plug_init_mode, :runtime

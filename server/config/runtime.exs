import Config

# Production configuration is read from the environment at boot, so no secrets
# live in the repo.
if config_env() == :prod do
  secret_key_base =
    System.get_env("SECRET_KEY_BASE") ||
      raise "SECRET_KEY_BASE is not set — generate one with `mix phx.gen.secret`"

  port = String.to_integer(System.get_env("PORT") || "4000")

  config :idaptik_server, IdaptikServerWeb.Endpoint,
    http: [ip: {0, 0, 0, 0, 0, 0, 0, 0}, port: port],
    secret_key_base: secret_key_base,
    server: true
end

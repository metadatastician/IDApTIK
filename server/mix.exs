defmodule IdaptikServer.MixProject do
  use Mix.Project

  # The multiplayer / session layer (ADR-0002): a headless realtime backend.
  # Phoenix Channels over Bandit — no LiveView. Rust owns gameplay truth; this
  # coordinates sessions, pairs the two asymmetric roles, and relays intent.

  def project do
    [
      app: :idaptik_server,
      version: "0.1.0",
      elixir: "~> 1.19",
      elixirc_paths: elixirc_paths(Mix.env()),
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      mod: {IdaptikServer.Application, []},
      extra_applications: [:logger, :runtime_tools]
    ]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  defp deps do
    [
      {:phoenix, "~> 1.7"},
      # Bandit is Phoenix's default HTTP/WebSocket adapter; declared explicitly so
      # the choice is visible and pinned.
      {:bandit, "~> 1.11"},
      {:phoenix_pubsub, "~> 2.1"},
      {:jason, "~> 1.4"}
      # Deliberately NOT phoenix_live_view — the game UI is the Rust client.
    ]
  end
end

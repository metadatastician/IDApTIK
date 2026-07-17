defmodule IdaptikServerWeb.SessionChannel do
  @moduledoc """
  One game session, shared by the two asymmetric roles.

  The **infiltrator** moves through the world; the **hacker** manipulates it
  remotely. They join the same `session:<id>` topic from opposite ends, and this
  channel relays each side's messages to the other. It carries intent between the
  clients — the Rust core remains the authority on what the world actually does
  (ADR-0002, ADR-0003).

  ## The typed stream (ADR-0005)

  The scenario's whole I/O surface is two serde enums in
  `crates/idaptik-core/src/scenario/{command,event}.rs`: `Command` in (tagged
  `"cmd"`) and `Event` out (tagged `"event"`). This channel relays both **verbatim**
  — the payload of a `"command"` or `"event"` message *is* the Rust JSON, byte-
  preserving, so what one client sends is exactly what the other's serde
  deserializes. Elixir never interprets the payload beyond reading the tag: no
  scoring, no FSM, no tick math live here (ADR-0005 records the relay-only
  lockstep topology and the migration path to a Rust-authoritative host).

  Role enforcement is a pure routing table over the tag: body verbs belong to the
  infiltrator, uplink/pivot verbs to the hacker, session/test immediates to either
  seat. An optional integer `"seq"` (a relay envelope field, stripped before
  relaying) lets the channel drop duplicate or out-of-order commands gracefully —
  stale sends are acknowledged, not crashed on, and never relayed.
  """

  use Phoenix.Channel

  @roles ~w(infiltrator hacker)

  # Which seat may send each `Command` variant (the `"cmd"` tag of the Rust
  # enum). This is routing, not rules: the sim itself decides what a command
  # *does* — this table only says whose hands are on which controls.
  @command_roles %{
    # The infiltrator's body verbs.
    "SetButton" => "infiltrator",
    "Jump" => "infiltrator",
    "Interact" => "infiltrator",
    "ThrowUsb" => "infiltrator",
    # The hacker's uplink and pivot verbs (pivots are hacker-side: they move
    # where the hacker plays *from*, never the body).
    "Uplink" => "hacker",
    "Pivot" => "hacker",
    "Unpivot" => "hacker",
    # Session and test immediates — either seat.
    "Pause" => :either,
    "Restart" => :either,
    "ForceCrisis" => :either,
    "ForceExtract" => :either,
    "ForceFail" => :either
  }

  @doc """
  The seat allowed to send a `Command` variant: `"infiltrator"`, `"hacker"`,
  `:either`, or `:unknown` for a tag that is not a `Command` at all.

  Public so tests (and any tooling replaying a fixture stream) can route each
  command from the correct side without duplicating the table.
  """
  def sender_for(cmd_tag), do: Map.get(@command_roles, cmd_tag, :unknown)

  @impl true
  def join("session:" <> _id, %{"role" => role}, socket) when role in @roles do
    socket =
      socket
      |> assign(:role, role)
      |> assign(:last_seq, nil)

    send(self(), :after_join)
    {:ok, %{role: role}, socket}
  end

  def join("session:" <> _id, _params, _socket) do
    {:error, %{reason: "join requires role: \"infiltrator\" or \"hacker\""}}
  end

  @impl true
  def handle_info(:after_join, socket) do
    # Tell the other side who just arrived.
    broadcast_from!(socket, "peer_joined", %{"role" => socket.assigns.role})
    {:noreply, socket}
  end

  # A typed `Command` (the Rust wire enum, tagged "cmd") -> the other seat.
  # The payload is relayed verbatim except for the optional "seq" envelope key.
  @impl true
  def handle_in("command", payload, socket) when is_map(payload) do
    with {:ok, tag} <- command_tag(payload),
         :ok <- authorize(tag, socket.assigns.role) do
      case note_seq(payload, socket) do
        {:fresh, socket} ->
          broadcast_from!(socket, "command", Map.delete(payload, "seq"))
          {:reply, {:ok, %{"relayed" => true}}, socket}

        {:stale, socket} ->
          # A duplicate or out-of-order send is a fact of networks, not a
          # protocol violation: acknowledge it, drop it, relay nothing.
          {:reply, {:ok, %{"relayed" => false, "reason" => "stale_or_duplicate"}}, socket}
      end
    else
      {:error, reason} -> {:reply, {:error, %{"reason" => reason}}, socket}
    end
  end

  def handle_in("command", _payload, socket) do
    {:reply, {:error, %{"reason" => "command payload must be a JSON object"}}, socket}
  end

  # A typed `Event` (the Rust wire enum, tagged "event") -> the other seat,
  # verbatim. Events are produced by the deterministic sims, not authored by a
  # seat, so both roles may publish them (ADR-0005: lockstep cross-feed).
  def handle_in("event", %{"event" => tag} = payload, socket) when is_binary(tag) do
    broadcast_from!(socket, "event", payload)
    {:reply, {:ok, %{"relayed" => true}}, socket}
  end

  def handle_in("event", _payload, socket) do
    {:reply, {:error, %{"reason" => "event payload must carry the \"event\" tag"}}, socket}
  end

  # Legacy freeform relays, kept for the pre-typed clients. New code speaks
  # "command"/"event" above.
  def handle_in("intent", payload, socket) do
    relay("intent", payload, socket)
  end

  def handle_in("hacker_action", payload, socket) do
    relay("hacker_action", payload, socket)
  end

  # Lightweight liveness check.
  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{"pong" => true}}, socket}
  end

  @impl true
  def terminate(_reason, socket) do
    # Tell the other side this seat left (best effort — an abrupt disconnect
    # may skip terminate; presence proper is a later milestone).
    if socket.joined do
      broadcast_from!(socket, "peer_left", %{"role" => socket.assigns.role})
    end

    :ok
  end

  # -- helpers ---------------------------------------------------------------

  defp command_tag(%{"cmd" => tag}) when is_binary(tag) do
    if Map.has_key?(@command_roles, tag) do
      {:ok, tag}
    else
      {:error, "unknown command: #{inspect(tag)}"}
    end
  end

  defp command_tag(_payload), do: {:error, "command payload must carry the \"cmd\" tag"}

  defp authorize(tag, role) do
    case @command_roles do
      %{^tag => :either} -> :ok
      %{^tag => ^role} -> :ok
      %{^tag => owner} -> {:error, "#{tag} is a #{owner} command (you joined as #{role})"}
    end
  end

  # Track the optional "seq" envelope: a fresh (strictly increasing) sequence
  # number is relayed; a duplicate or out-of-order one is dropped gracefully.
  # Commands without "seq" are always fresh — ordering is then the transport's.
  defp note_seq(%{"seq" => seq}, socket) when is_integer(seq) do
    case socket.assigns.last_seq do
      last when is_integer(last) and seq <= last -> {:stale, socket}
      _ -> {:fresh, assign(socket, :last_seq, seq)}
    end
  end

  defp note_seq(_payload, socket), do: {:fresh, socket}

  # Send an event to the *other* participant, tagged with who it came from.
  defp relay(event, payload, socket) do
    payload = Map.put(payload, "from", socket.assigns.role)
    broadcast_from!(socket, event, payload)
    {:noreply, socket}
  end
end

defmodule IdaptikServerWeb.SessionChannel do
  @moduledoc """
  One game session, shared by the two asymmetric roles.

  The **infiltrator** moves through the world; the **hacker** manipulates it
  remotely. They join the same `session:<id>` topic from opposite ends, and this
  channel relays each side's messages to the other. It carries intent between the
  clients — the Rust core remains the authority on what the world actually does
  (ADR-0002, ADR-0003).
  """

  use Phoenix.Channel

  @roles ~w(infiltrator hacker)

  @impl true
  def join("session:" <> _id, %{"role" => role}, socket) when role in @roles do
    socket = assign(socket, :role, role)
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

  # Infiltrator movement/intent -> hacker.
  @impl true
  def handle_in("intent", payload, socket) do
    relay("intent", payload, socket)
  end

  # Hacker action (open door, cut power, override) -> infiltrator.
  def handle_in("hacker_action", payload, socket) do
    relay("hacker_action", payload, socket)
  end

  # Lightweight liveness check.
  def handle_in("ping", _payload, socket) do
    {:reply, {:ok, %{"pong" => true}}, socket}
  end

  # Send an event to the *other* participant, tagged with who it came from.
  defp relay(event, payload, socket) do
    payload = Map.put(payload, "from", socket.assigns.role)
    broadcast_from!(socket, event, payload)
    {:noreply, socket}
  end
end

defmodule IdaptikServerWeb.SessionChannelTest do
  use IdaptikServerWeb.ChannelCase, async: true

  alias IdaptikServerWeb.SessionChannel
  alias IdaptikServerWeb.UserSocket

  # The cross-language wire fixture, shared with the Rust round-trip test
  # (crates/idaptik-core/tests/session_relay_fixture.rs). See ADR-0005.
  @fixtures Path.expand("../../../../fixtures/session_relay", __DIR__)

  defp join!(session, role) do
    {:ok, reply, socket} =
      UserSocket
      |> socket()
      |> subscribe_and_join(SessionChannel, "session:" <> session, %{"role" => role})

    {reply, socket}
  end

  defp fixture!(name) do
    @fixtures |> Path.join(name) |> File.read!() |> Jason.decode!()
  end

  describe "join" do
    test "accepts each role and announces it to the peer" do
      {reply, _infil} = join!("j1", "infiltrator")
      assert reply == %{role: "infiltrator"}

      {reply, _hacker} = join!("j1", "hacker")
      assert reply == %{role: "hacker"}
      # The infiltrator (already subscribed) hears the hacker arrive.
      assert_broadcast "peer_joined", %{"role" => "hacker"}
    end

    test "rejects a join without a role" do
      assert {:error, %{reason: reason}} =
               UserSocket
               |> socket()
               |> subscribe_and_join(SessionChannel, "session:j2", %{})

      assert reason =~ "role"
    end

    test "rejects an unknown role" do
      assert {:error, _} =
               UserSocket
               |> socket()
               |> subscribe_and_join(SessionChannel, "session:j3", %{"role" => "billy"})
    end

    test "a leaving peer is announced" do
      {_reply, infil} = join!("j4", "infiltrator")
      {_reply, _hacker} = join!("j4", "hacker")
      assert_broadcast "peer_joined", %{"role" => "hacker"}

      Process.unlink(infil.channel_pid)
      leave(infil)
      assert_broadcast "peer_left", %{"role" => "infiltrator"}
    end
  end

  describe "ping" do
    test "answers pong" do
      {_reply, socket} = join!("p1", "hacker")
      ref = push(socket, "ping", %{})
      assert_reply ref, :ok, %{"pong" => true}
    end
  end

  describe "command relay" do
    test "delivers a hacker Pivot to the infiltrator verbatim" do
      {_reply, _infil} = join!("c1", "infiltrator")
      {_reply, hacker} = join!("c1", "hacker")

      payload = %{"cmd" => "Pivot", "target" => "Bridge"}
      ref = push(hacker, "command", payload)
      assert_reply ref, :ok, %{"relayed" => true}

      # The infiltrator's channel pushes the identical payload to its client.
      assert_push "command", ^payload
    end

    test "delivers infiltrator body verbs to the hacker verbatim" do
      {_reply, infil} = join!("c2", "infiltrator")

      payload = %{"cmd" => "SetButton", "button" => "Right", "down" => true}
      ref = push(infil, "command", payload)
      assert_reply ref, :ok, %{"relayed" => true}
      assert_broadcast "command", ^payload
    end

    test "either seat may send session immediates" do
      {_reply, infil} = join!("c3", "infiltrator")
      {_reply, hacker} = join!("c3", "hacker")

      for {socket, payload} <- [
            {infil, %{"cmd" => "Pause", "on" => true}},
            {hacker, %{"cmd" => "Restart"}},
            {infil, %{"cmd" => "ForceExtract", "method" => "ServiceExit"}}
          ] do
        ref = push(socket, "command", payload)
        assert_reply ref, :ok, %{"relayed" => true}
        assert_broadcast "command", ^payload
      end
    end
  end

  describe "role enforcement" do
    test "the infiltrator cannot pivot" do
      {_reply, infil} = join!("r1", "infiltrator")

      ref = push(infil, "command", %{"cmd" => "Pivot", "target" => "GridJump"})
      assert_reply ref, :error, %{"reason" => reason}
      assert reason =~ "hacker"
      refute_broadcast "command", _
    end

    test "the hacker has no body" do
      {_reply, hacker} = join!("r2", "hacker")

      for payload <- [
            %{"cmd" => "Jump"},
            %{"cmd" => "Interact"},
            %{"cmd" => "ThrowUsb"},
            %{"cmd" => "SetButton", "button" => "Left", "down" => true}
          ] do
        ref = push(hacker, "command", payload)
        assert_reply ref, :error, %{"reason" => reason}
        assert reason =~ "infiltrator"
      end

      refute_broadcast "command", _
    end

    test "an unknown or untagged command is refused, not relayed" do
      {_reply, hacker} = join!("r3", "hacker")

      ref = push(hacker, "command", %{"cmd" => "BecomeAdmin"})
      assert_reply ref, :error, %{"reason" => reason}
      assert reason =~ "unknown command"

      ref = push(hacker, "command", %{"target" => "Bridge"})
      assert_reply ref, :error, %{"reason" => reason2}
      assert reason2 =~ "cmd"

      refute_broadcast "command", _
    end
  end

  describe "sequence handling" do
    test "a duplicate command is acknowledged but dropped" do
      {_reply, hacker} = join!("s1", "hacker")
      payload = %{"cmd" => "Unpivot", "seq" => 7}

      ref = push(hacker, "command", payload)
      assert_reply ref, :ok, %{"relayed" => true}
      # The relay envelope ("seq") is stripped; the Command JSON is untouched.
      assert_broadcast "command", %{"cmd" => "Unpivot"} = relayed
      refute Map.has_key?(relayed, "seq")

      ref = push(hacker, "command", payload)
      assert_reply ref, :ok, %{"relayed" => false, "reason" => "stale_or_duplicate"}
      refute_broadcast "command", _
    end

    test "an out-of-order command is acknowledged but dropped" do
      {_reply, hacker} = join!("s2", "hacker")

      ref = push(hacker, "command", %{"cmd" => "Pivot", "target" => "IspOps", "seq" => 5})
      assert_reply ref, :ok, %{"relayed" => true}
      assert_broadcast "command", %{"cmd" => "Pivot"}

      ref = push(hacker, "command", %{"cmd" => "Unpivot", "seq" => 3})
      assert_reply ref, :ok, %{"relayed" => false}
      refute_broadcast "command", _
    end

    test "each seat's sequence is independent" do
      {_reply, infil} = join!("s3", "infiltrator")
      {_reply, hacker} = join!("s3", "hacker")

      ref = push(hacker, "command", %{"cmd" => "Unpivot", "seq" => 9})
      assert_reply ref, :ok, %{"relayed" => true}

      # The infiltrator's own counter starts fresh; a lower seq still relays.
      ref = push(infil, "command", %{"cmd" => "Jump", "seq" => 1})
      assert_reply ref, :ok, %{"relayed" => true}
    end
  end

  describe "event relay" do
    test "delivers a typed Event to the other seat verbatim" do
      {_reply, _infil} = join!("e1", "infiltrator")
      {_reply, hacker} = join!("e1", "hacker")

      payload = %{"event" => "PivotOpened", "host" => "ops.isp.net", "hops" => 1}
      ref = push(hacker, "event", payload)
      assert_reply ref, :ok, %{"relayed" => true}
      assert_push "event", ^payload
    end

    test "an untagged event is refused" do
      {_reply, hacker} = join!("e2", "hacker")

      ref = push(hacker, "event", %{"hops" => 1})
      assert_reply ref, :error, %{"reason" => reason}
      assert reason =~ "event"
      refute_broadcast "event", _
    end
  end

  describe "cross-language fixture pass-through" do
    test "every fixture Command relays byte-preserving from its allowed seat" do
      {_reply, infil} = join!("f1", "infiltrator")
      {_reply, hacker} = join!("f1", "hacker")
      assert_broadcast "peer_joined", %{"role" => "hacker"}

      for command <- fixture!("commands.json") do
        socket =
          case SessionChannel.sender_for(command["cmd"]) do
            "infiltrator" -> infil
            "hacker" -> hacker
            :either -> hacker
          end

        ref = push(socket, "command", command)
        assert_reply ref, :ok, %{"relayed" => true}
        # Exactly the decoded fixture value comes out the other side: the relay
        # added, removed, and rewrote nothing.
        assert_broadcast "command", ^command
      end
    end

    test "every captured Event relays byte-preserving" do
      {_reply, _infil} = join!("f2", "infiltrator")
      {_reply, hacker} = join!("f2", "hacker")
      assert_broadcast "peer_joined", %{"role" => "hacker"}

      for event <- fixture!("events.json") do
        ref = push(hacker, "event", event)
        assert_reply ref, :ok, %{"relayed" => true}
        assert_push "event", ^event
      end
    end
  end

  describe "legacy relay" do
    test "intent and hacker_action still fan out, tagged with the sender" do
      {_reply, infil} = join!("l1", "infiltrator")
      {_reply, hacker} = join!("l1", "hacker")

      push(infil, "intent", %{"move" => "left"})
      assert_broadcast "intent", %{"move" => "left", "from" => "infiltrator"}

      push(hacker, "hacker_action", %{"action" => "cut_power"})
      assert_broadcast "hacker_action", %{"action" => "cut_power", "from" => "hacker"}
    end
  end
end

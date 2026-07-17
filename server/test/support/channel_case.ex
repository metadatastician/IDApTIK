defmodule IdaptikServerWeb.ChannelCase do
  @moduledoc """
  Test case template for channel tests: imports `Phoenix.ChannelTest` against
  this app's endpoint.
  """

  use ExUnit.CaseTemplate

  using do
    quote do
      import Phoenix.ChannelTest
      import IdaptikServerWeb.ChannelCase

      @endpoint IdaptikServerWeb.Endpoint
    end
  end
end

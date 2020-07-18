defmodule Server.Transform do
  alias Server.Transform.Options
  alias Server.Transform.Port

  def transform(%Options{} = options, caller \\ self()) do
    spawn(fn ->
      port = Port.open()

      spawn(fn ->
        Port.send_options(port, options)
        Port.send_done(port)
      end)

      # blocks
      Port.port_receive(port, caller)
    end)

    :ok
  end
end

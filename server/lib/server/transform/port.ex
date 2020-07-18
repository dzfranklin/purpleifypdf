defmodule Server.Transform.Port do
  alias Server.Transform
  alias Transform.Options
  require Logger

  def open(bin \\ Application.get_env(:server, __MODULE__)[:bin]),
    do: Port.open({:spawn_executable, bin}, [:binary, packet: 4])

  def send_options(port, %Options{} = options), do: port_send(port, "OPTS", options)
  def send_done(port), do: port_send(port, "DONE")

  def port_receive(port, caller) do
    receive do
      {^port, {:data, data}} ->
        msg = parse_received(data)
        send(caller, msg)

        case msg do
          {:error, _} = msg ->
            Logger.warn("Closing because of error #{inspect(msg)}")

          _ ->
            port_receive(port, caller)
        end
    after
      :timer.minutes(30) ->
        Logger.warn("Closing because of timeout")
        send(caller, {:error, :timeout})
    end
  end

  defp parse_received(<<category::binary-size(4), data::binary>>),
    do: {parse_category(category), Jason.decode!(data)}

  defp parse_category("ERRR"), do: :error
  defp parse_category("STAT"), do: :status
  defp parse_category("DONE"), do: :done

  defp port_send(port, category) do
    with {:ok, category} <- validate_category(category) do
      Port.command(port, category)
    end
  end

  defp port_send(port, category, data) do
    with {:ok, category} <- validate_category(category),
         {:ok, data} <- Jason.encode(data) do
      Port.command(port, category <> data)
    end
  end

  defp validate_category(category_str) when byte_size(category_str) == 4, do: {:ok, category_str}
  defp validate_category(_), do: {:error, :invalid_category}
end

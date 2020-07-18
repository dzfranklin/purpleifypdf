defmodule ServerWeb.TransformController do
  use ServerWeb, :controller
  alias Server.Transform

  def index(conn, _) do
    render(conn, "index.html")
  end

  def upload(conn, %{
        "transform" => %{
          "input" => %Plug.Upload{filename: filename, path: path},
          "quality" => quality,
          "background_color" => background_color
        }
      }) do
    with {:ok, quality} <- parse_quality(quality),
         {:ok, background_color} <- parse_background_color(background_color) do
      out_file = tempfile()

      Transform.transform(%Transform.Options{
        quality: quality,
        background_color: background_color,
        in_file: path,
        out_file: out_file
      })

      receive do
        {:done, _} ->
          conn
          |> send_download({:file, out_file},
            filename: encode_filename(filename),
            encode: false
          )
          |> put_flash(:info, "Downloading")
          |> redirect(to: Routes.transform_path(conn, :index))

        {:error, %{"message" => message}} ->
          conn
          |> put_flash(:error, "INTERNAL ERROR: #{message}")
          |> redirect(to: Routes.transform_path(conn, :index))
      after
        :timer.minutes(30) ->
          conn
          |> put_flash(:error, "Transformation timed out")
          |> redirect(to: Routes.transform_path(conn, :index))
      end
    end
  end

  defp encode_filename(filename) do
    filename
    |> URI.encode_www_form()
    |> String.replace("+", "%20")
  end

  defp parse_quality("extreme"), do: {:ok, "Extreme"}
  defp parse_quality("high"), do: {:ok, "High"}
  defp parse_quality("normal"), do: {:ok, "Normal"}
  defp parse_quality("low"), do: {:ok, "Low"}
  defp parse_quality(_), do: {:error, "Invalid quality"}

  defp parse_background_color(<<?#, r::binary-size(2), g::binary-size(2), b::binary-size(2)>>) do
    with {r, _} <- Integer.parse(r, 16),
         {g, _} <- Integer.parse(g, 16),
         {b, _} <- Integer.parse(b, 16) do
      {:ok,
       %Transform.Color{
         r: r,
         g: g,
         b: b
       }}
    else
      _ -> {:error, "Invalid background color"}
    end
  end

  defp tempfile do
    random_part =
      :math.pow(10, 9)
      |> round()
      |> :random.uniform()
      |> Integer.to_string()
      |> Base.encode64()

    time_part =
      DateTime.utc_now()
      |> DateTime.to_unix()
      |> Integer.to_string()

    Path.join(System.tmp_dir!(), "#{time_part}_#{random_part}")
  end
end

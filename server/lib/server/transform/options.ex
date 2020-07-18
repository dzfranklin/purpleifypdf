defmodule Server.Transform.Options do
  alias Server.Transform.Color

  @derive Jason.Encoder
  defstruct quality: "High",
            background_color: %Color{r: 226, g: 97, b: 255},
            in_file: nil,
            out_file: nil
end

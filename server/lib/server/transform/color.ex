defmodule Server.Transform.Color do
  @derive Jason.Encoder
  defstruct r: 255, g: 255, b: 255
end

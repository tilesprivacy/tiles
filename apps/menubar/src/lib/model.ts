const PROVIDERS: [RegExp, string][] = [
  [/^(gemma|gemini)/, "google"],
  [/^(gpt|o[134])/, "openai"],
  [/^qw(en|q)/, "alibaba"],
  [/^(llama|codellama)/, "meta"],
  [/^(mistral|mixtral|magistral|devstral|codestral)/, "mistral"],
  [/^deepseek/, "deepseek"],
  [/^kimi/, "moonshotai"],
  [/^(smol|hf)/, "huggingface"],
  [/^(nemotron|nvidia)/, "nvidia"],
  [/^glm/, "zai"],
  [/^minimax/, "minimax"],
];

export type Model = {
  /** the spec with its path, quant suffix and format suffix taken off */
  name: string;
  provider: string;
  /** Q4, Q8, F16, whatever the weights were packed at */
  quant: string | null;
  format: string | null;
};

export function describe(spec: string): Model {
  const base = spec.split("/").at(-1) ?? spec;
  const [stem, tag] = base.split(":");
  const lower = stem.toLowerCase();

  const format = /gguf/i.test(base) ? "GGUF" : /mlx/i.test(base) ? "MLX" : null;
  const quant = readQuant(tag ?? stem);

  return {
    // the format and the quant have their own chips, so they leave the name
    name: stem.replace(/[-_.]?(gguf|mlx)$/i, "").replace(/[-_.]?q\d+(_[a-z0-9]+)*$/i, ""),
    provider: PROVIDERS.find(([pattern]) => pattern.test(lower))?.[1] ?? "generic",
    quant,
    format,
  };
}

function readQuant(text: string): string | null {
  const packed = /\bq(\d+)(?:_[a-z0-9]+)*\b/i.exec(text);
  if (packed) return `Q${packed[1]}`;

  const float = /\b(bf16|f16|f32)\b/i.exec(text);
  return float ? float[1].toUpperCase() : null;
}

/** 4095 reads as 4K, and the panel has no room for the exact figure */
export function contextLabel(tokens: number): string {
  return tokens >= 1024 ? `${Math.round(tokens / 1024)}K` : `${tokens}`;
}

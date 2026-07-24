const TICKS = "▁▂▃▄▅▆▇█";

export function sparkline(
  values: number[],
  params?: { width?: number; min?: number; max?: number },
): string {
  const width = Math.max(1, params?.width ?? values.length);
  if (values.length === 0) return "".padEnd(width, " ");

  const sampled = sampleToWidth(values, width);
  const min = params?.min ?? Math.min(...sampled);
  const max = params?.max ?? Math.max(...sampled);

  if (max <= min) return TICKS[0].repeat(width);

  return sampled
    .map((v) => {
      const t = (v - min) / (max - min);
      const idx = Math.max(
        0,
        Math.min(TICKS.length - 1, Math.round(t * (TICKS.length - 1))),
      );
      return TICKS[idx];
    })
    .join("");
}

function sampleToWidth(values: number[], width: number): number[] {
  if (values.length === width) return values.slice();
  if (values.length < width) {
    const pad = new Array(width - values.length).fill(0);
    return [...pad, ...values];
  }

  const out: number[] = [];
  for (let i = 0; i < width; i++) {
    const start = Math.floor((i * values.length) / width);
    const end = Math.floor(((i + 1) * values.length) / width);
    const slice = values.slice(start, Math.max(start + 1, end));
    const sum = slice.reduce((a, b) => a + b, 0);
    out.push(sum);
  }
  return out;
}

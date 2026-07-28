import blessed from "blessed";

// Lightweight, non-blocking modal input (doesn't stop WS event processing).
export async function promptLine(params: {
  screen: blessed.Widgets.Screen;
  title: string;
  hint: string;
  initial?: string;
}): Promise<string | undefined> {
  const overlay = blessed.box({
    parent: params.screen,
    top: "center",
    left: "center",
    width: "80%",
    height: 7,
    border: "line",
    label: ` ${params.title} `,
    style: { border: { fg: "cyan" } },
  });

  blessed.text({
    parent: overlay,
    top: 1,
    left: 2,
    right: 2,
    height: 2,
    content: params.hint,
    style: { fg: "white" },
  });

  const input = blessed.textbox({
    parent: overlay,
    top: 3,
    left: 2,
    right: 2,
    height: 1,
    inputOnFocus: true,
    value: params.initial ?? "",
    border: "line",
    style: {
      border: { fg: "gray" },
      focus: { border: { fg: "yellow" } },
    },
  });

  blessed.text({
    parent: overlay,
    top: 5,
    left: 2,
    right: 2,
    height: 1,
    content: "Enter: apply   Esc: cancel",
    style: { fg: "gray" },
  });

  params.screen.render();
  input.focus();

  return await new Promise<string | undefined>((resolve) => {
    const cleanup = (result: string | undefined) => {
      input.removeAllListeners();
      overlay.detach();
      params.screen.render();
      resolve(result);
    };

    input.key(["escape"], () => cleanup(undefined));
    input.on("submit", (value) => cleanup(String(value ?? "")));
    input.readInput();
  });
}

export function parseKeyValueFilters(line: string): {
  network?: string;
  category?: string;
  query?: string;
} {
  const out: { network?: string; category?: string; query?: string } = {};
  for (const token of line.split(/\s+/).filter(Boolean)) {
    const [k, ...rest] = token.split("=");
    const v = rest.join("=").trim();
    if (!k) continue;
    if (k === "network") out.network = v || undefined;
    if (k === "category") out.category = v || undefined;
    if (k === "query") out.query = v || undefined;
  }
  return out;
}

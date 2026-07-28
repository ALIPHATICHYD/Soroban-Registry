"use client";

import { useState } from "react";
import CodeCopyButton from "./CodeCopyButton";
import { useCopy } from "@/hooks/useCopy";

interface CodeRunnerProps {
  initialCode: string;
  language: "javascript" | "rust";
  copyAnalytics?: {
    contractId?: string;
    exampleId?: string;
    exampleTitle?: string;
  };
}

export default function CodeRunner({
  initialCode,
  language,
  copyAnalytics,
}: CodeRunnerProps) {
  const [code, setCode] = useState(initialCode);
  const [output, setOutput] = useState<string>("");
  const [isRunning, setIsRunning] = useState(false);
  // Shared copy hook handles clipboard state and analytics event emission.
  const { copy, copied, isCopying } = useCopy();

  const handleCopyCode = async () => {
    await copy(code, {
      successEventName: "contract_code_copied",
      failureEventName: "contract_code_copy_failed",
      successMessage: "Code copied",
      failureMessage: "Unable to copy code",
      analyticsParams: {
        // Context passed from parent so copied events are tied to contract/example.
        ...copyAnalytics,
        language,
      },
    });
  };

  const runCode = async () => {
    if (language !== "javascript") {
      setOutput("Running Rust code in the browser is not supported yet.");
      return;
    }

    setIsRunning(true);
    setOutput("");

    try {
      // Execute user code in a sandboxed iframe to prevent XSS and DOM access.
      // The iframe has no access to the parent page's cookies, DOM, or scripts.
      const result = await new Promise<string>((resolve, reject) => {
        const iframe = document.createElement("iframe");
        iframe.sandbox.add("allow-scripts");
        iframe.style.display = "none";
        document.body.appendChild(iframe);

        const timeout = setTimeout(() => {
          document.body.removeChild(iframe);
          reject(new Error("Execution timed out (10s)"));
        }, 10_000);

        const handler = (event: MessageEvent) => {
          if (event.source !== iframe.contentWindow) return;
          clearTimeout(timeout);
          window.removeEventListener("message", handler);
          document.body.removeChild(iframe);
          if (event.data?.error) {
            reject(new Error(event.data.error));
          } else {
            resolve(
              event.data?.logs?.join("\n") ||
                "Code executed successfully (no output).",
            );
          }
        };

        window.addEventListener("message", handler);

        const scriptContent = `
          <script>
            (async () => {
              const logs = [];
              const console = {
                log: (...args) => logs.push(args.map(String).join(' ')),
                error: (...args) => logs.push('ERROR: ' + args.map(String).join(' ')),
                warn: (...args) => logs.push('WARN: ' + args.map(String).join(' ')),
              };
              try {
                ${code}
              } catch (e) {
                console.error(e?.message || e);
              }
              parent.postMessage({ logs }, '*');
            })().catch(e => parent.postMessage({ error: e?.message || String(e) }, '*'));
          <\/script>
        `;

        iframe.srcdoc = scriptContent;
      });

      setOutput(result);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : "Unknown error";
      setOutput(`Execution Error: ${message}`);
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="relative rounded-lg overflow-hidden border border-border">
        <div className="bg-accent px-4 py-2 flex items-center justify-between border-b border-border">
          <span className="text-xs font-mono text-muted-foreground uppercase">
            {language}
          </span>
          <div className="flex items-center gap-2">
            <CodeCopyButton
              onCopy={handleCopyCode}
              copied={copied}
              disabled={isCopying}
            />
            {language === "javascript" && (
              <button
                onClick={runCode}
                disabled={isRunning}
                className={`px-3 py-1 rounded-md text-xs font-medium text-white transition-colors ${
                  isRunning
                    ? "bg-muted text-muted-foreground cursor-not-allowed"
                    : "bg-green-600 hover:bg-green-700"
                }`}
              >
                {isRunning ? "Running..." : "Run Code"}
              </button>
            )}
          </div>
        </div>
        <textarea
          aria-label="Code editor"
          value={code}
          onChange={(e) => setCode(e.target.value)}
          className="w-full h-64 p-4 font-mono text-sm bg-surface text-foreground focus:outline-none resize-none"
          spellCheck={false}
        />
      </div>

      {(output || isRunning) && (
        <div className="rounded-lg bg-surface text-foreground p-4 font-mono text-sm overflow-x-auto">
          <div className="text-muted-foreground text-xs mb-2 uppercase">
            Output
          </div>
          <pre className="whitespace-pre-wrap">{output}</pre>
        </div>
      )}
    </div>
  );
}

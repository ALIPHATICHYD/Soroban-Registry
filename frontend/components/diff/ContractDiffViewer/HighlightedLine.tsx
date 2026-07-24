'use client';


const RUST_KW = new Set([
  "fn", "let", "mut", "pub", "struct", "enum", "impl", "trait", "use", "mod",
  "match", "if", "else", "for", "while", "loop", "return", "async", "await",
  "where", "crate", "Self", "self", "const", "static", "type", "move", "ref",
  "in", "as", "unsafe", "extern", "dyn", "Box", "Option", "Result", "Some",
  "None", "Ok", "Err", "true", "false",
]);

function tokenClass(token: string, inComment: boolean): string {
    if (inComment) return "text-emerald-500 dark:text-emerald-400";
    if (token.startsWith('"') || (token.startsWith("'") && token.length >= 3))
        return "text-amber-600 dark:text-amber-300";
    if (/^\d/.test(token)) return "text-purple-600 dark:text-purple-300";
    if (RUST_KW.has(token.replace(/[^A-Za-z0-9_]/g, "")))
        return "text-sky-600 dark:text-sky-300 font-semibold";
    if (/^[A-Z][A-Za-z0-9_]*$/.test(token)) return "text-teal-600 dark:text-teal-300";
    return "";
}

export function HighlightedLine({ value }: { value: string }) {
    const tokens = value.split(/(\s+)/);
    const rendered = tokens.reduce<{ inComment: boolean; nodes: React.ReactNode[] }>(
        ({ inComment, nodes }, tok, i) => {
            const nowInComment = inComment || tok.includes("//");
            const cls = tokenClass(tok, nowInComment);
            const node = cls ? (
                <span key={i} className={cls}>
                    {tok}
                </span>
            ) : (
                tok
            );
            return { inComment: nowInComment, nodes: [...nodes, node] };
        },
        { inComment: false, nodes: [] }
    );
    return <>{rendered.nodes}</>;
}

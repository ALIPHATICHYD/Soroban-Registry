"use client";

import type { QueryCondition, QueryNode, QueryOperator } from "@/types";

type Token =
  | { type: "op"; op: QueryOperator }
  | { type: "tag"; value: string }
  | { type: "term"; value: string }
  | { type: "lparen" }
  | { type: "rparen" };

export type ParsedAdvancedQuery = {
  queryNode: QueryNode | null;
  tags: string[];
  usesOr: boolean;
  cleanedSimpleQuery: string;
};

function isWhitespace(char: string) {
  return char === " " || char === "\n" || char === "\t" || char === "\r";
}

function isOpToken(value: string): QueryOperator | null {
  const upper = value.toUpperCase();
  if (upper === "AND" || value === "&&") return "AND";
  if (upper === "OR" || value === "||") return "OR";
  return null;
}

function readQuoted(input: string, start: number) {
  let index = start;
  let value = "";
  while (index < input.length) {
    const char = input[index];
    if (char === "\\" && index + 1 < input.length) {
      value += input[index + 1];
      index += 2;
      continue;
    }
    if (char === '"') {
      return { value, next: index + 1 };
    }
    value += char;
    index += 1;
  }
  return { value, next: index };
}

function readBare(input: string, start: number) {
  let index = start;
  let value = "";
  while (index < input.length) {
    const char = input[index];
    if (isWhitespace(char) || char === "(" || char === ")") break;
    value += char;
    index += 1;
  }
  return { value, next: index };
}

function tokenize(input: string): Token[] {
  const tokens: Token[] = [];
  let index = 0;

  while (index < input.length) {
    const char = input[index];
    if (isWhitespace(char)) {
      index += 1;
      continue;
    }

    if (char === "(") {
      tokens.push({ type: "lparen" });
      index += 1;
      continue;
    }
    if (char === ")") {
      tokens.push({ type: "rparen" });
      index += 1;
      continue;
    }

    // #tag shorthand
    if (char === "#") {
      const { value, next } = readBare(input, index + 1);
      if (value.trim()) {
        tokens.push({ type: "tag", value });
      }
      index = next;
      continue;
    }

    const lowerRemainder = input.slice(index).toLowerCase();
    if (
      lowerRemainder.startsWith("tag:") ||
      lowerRemainder.startsWith("tags:")
    ) {
      const prefixLength = lowerRemainder.startsWith("tags:") ? 5 : 4;
      index += prefixLength;
      while (index < input.length && isWhitespace(input[index])) index += 1;
      if (index >= input.length) break;

      if (input[index] === '"') {
        const { value, next } = readQuoted(input, index + 1);
        if (value.trim()) tokens.push({ type: "tag", value });
        index = next;
        continue;
      }

      const { value, next } = readBare(input, index);
      if (value.trim()) tokens.push({ type: "tag", value });
      index = next;
      continue;
    }

    if (char === '"') {
      const { value, next } = readQuoted(input, index + 1);
      if (value.trim()) tokens.push({ type: "term", value });
      index = next;
      continue;
    }

    const { value, next } = readBare(input, index);
    const op = isOpToken(value);
    if (op) {
      tokens.push({ type: "op", op });
    } else if (value.trim()) {
      tokens.push({ type: "term", value });
    }
    index = next;
  }

  return tokens;
}

function makeCondition(
  field: string,
  operator: QueryCondition["operator"],
  value: string,
): QueryNode {
  return { field, operator, value };
}

function makeGroup(
  operator: QueryOperator,
  conditions: QueryNode[],
): QueryNode {
  if (conditions.length === 1) return conditions[0];
  return { operator, conditions };
}

function makeKeywordNode(text: string): QueryNode {
  const term = text.trim();
  if (!term) return makeCondition("name", "contains", "");
  return makeGroup("OR", [
    makeCondition("name", "contains", term),
    makeCondition("description", "contains", term),
    makeCondition("category", "contains", term),
    makeCondition("publisher", "contains", term),
    makeCondition("tag", "contains", term),
  ]);
}

function parseTokens(tokens: Token[]): QueryNode | null {
  let index = 0;

  const at = () => tokens[index];
  const consume = () => tokens[index++];

  const startsPrimary = (token: Token | undefined) =>
    token?.type === "term" || token?.type === "tag" || token?.type === "lparen";

  const parsePrimary = (): QueryNode | null => {
    const token = at();
    if (!token) return null;
    if (token.type === "lparen") {
      consume();
      const expr = parseOr();
      if (at()?.type === "rparen") consume();
      return expr;
    }
    if (token.type === "tag") {
      consume();
      return makeCondition("tag", "contains", token.value);
    }
    if (token.type === "term") {
      consume();
      return makeKeywordNode(token.value);
    }
    return null;
  };

  const parseAnd = (): QueryNode | null => {
    const first = parsePrimary();
    if (!first) return null;
    const nodes: QueryNode[] = [first];

    while (index < tokens.length) {
      const token = at();
      if (!token) break;
      if (token.type === "op" && token.op === "AND") {
        consume();
        const next = parsePrimary();
        if (next) nodes.push(next);
        continue;
      }
      if (startsPrimary(token)) {
        const next = parsePrimary();
        if (next) nodes.push(next);
        continue;
      }
      break;
    }

    return makeGroup("AND", nodes);
  };

  const parseOr = (): QueryNode | null => {
    const first = parseAnd();
    if (!first) return null;
    const nodes: QueryNode[] = [first];

    while (index < tokens.length) {
      const token = at();
      if (!token) break;
      if (token.type !== "op" || token.op !== "OR") break;
      consume();
      const next = parseAnd();
      if (next) nodes.push(next);
    }

    return makeGroup("OR", nodes);
  };

  return parseOr();
}

export function parseAdvancedContractQuery(input: string): ParsedAdvancedQuery {
  const raw = input ?? "";
  const tokens = tokenize(raw);

  const tags = tokens
    .filter(
      (token): token is Extract<Token, { type: "tag" }> => token.type === "tag",
    )
    .map((token) => token.value.trim())
    .filter(Boolean);

  const usesOr = tokens.some(
    (token) => token.type === "op" && token.op === "OR",
  );

  const cleanedSimpleQuery = tokens
    .filter(
      (token): token is Extract<Token, { type: "term" }> =>
        token.type === "term",
    )
    .map((token) => token.value.trim())
    .filter(Boolean)
    .join(" ")
    .trim();

  const queryNode = tokens.length > 0 ? parseTokens(tokens) : null;

  return { queryNode, tags, usesOr, cleanedSimpleQuery };
}

export function combineAdvancedQueryWithFilters(
  base: QueryNode,
  filters: {
    categories?: string[];
    networks?: Array<"mainnet" | "testnet" | "futurenet">;
    tags?: string[];
    author?: string;
    verified_only?: boolean;
    favorites_only?: boolean;
    favorites_list?: string[];
  },
): QueryNode {
  const andConditions: QueryNode[] = [base];

  if (filters.verified_only) {
    andConditions.push({ field: "verified", operator: "eq", value: true });
  }

  if (filters.favorites_only && filters.favorites_list?.length) {
    if (filters.favorites_list.length === 1) {
      andConditions.push({ field: 'contract_id', operator: 'eq', value: filters.favorites_list[0] });
    } else {
      andConditions.push({ field: 'contract_id', operator: 'in', value: filters.favorites_list });
    }
  }

  if (filters.author?.trim()) {
    andConditions.push({
      field: "publisher",
      operator: "contains",
      value: filters.author.trim(),
    });
  }

  if (filters.categories?.length) {
    if (filters.categories.length === 1) {
      andConditions.push({
        field: "category",
        operator: "eq",
        value: filters.categories[0],
      });
    } else {
      andConditions.push({
        field: "category",
        operator: "in",
        value: filters.categories,
      });
    }
  }

  if (filters.networks?.length) {
    if (filters.networks.length === 1) {
      andConditions.push({
        field: "network",
        operator: "eq",
        value: filters.networks[0],
      });
    } else {
      andConditions.push({
        field: "network",
        operator: "in",
        value: filters.networks,
      });
    }
  }

  if (filters.tags?.length) {
    if (filters.tags.length === 1) {
      andConditions.push({
        field: "tag",
        operator: "contains",
        value: filters.tags[0],
      });
    } else {
      andConditions.push({ field: "tag", operator: "in", value: filters.tags });
    }
  }

  return makeGroup("AND", andConditions);
}

import pino from "pino";

const LOG_LEVEL = process.env.LOG_LEVEL || "info";
const SERVICE_NAME = process.env.OTEL_SERVICE_NAME || "tagging-service";

const transport = process.env.LOG_PRETTY === "true"
  ? { target: "pino-pretty", options: { colorize: true } }
  : undefined;

export const logger = pino({
  name: SERVICE_NAME,
  level: LOG_LEVEL,
  transport,
  serializers: {
    req: (req) => ({
      method: req.method,
      url: req.url,
      headers: req.headers ? sanitizeHeaders(req.headers) : undefined,
    }),
    res: (res) => ({
      statusCode: res.statusCode,
    }),
    err: pino.stdSerializers.err,
    error: pino.stdSerializers.err,
  },
  redact: {
    paths: [
      "req.headers.authorization",
      "req.headers.cookie",
      "req.headers['x-api-key']",
      "req.body.password",
      "req.body.secret",
      "req.body.token",
      "req.body.api_key",
      "req.body.private_key",
      "req.body.credit_card",
      "req.body.ssn",
      "password",
      "secret",
      "token",
      "api_key",
      "private_key",
      "authorization",
      "credit_card",
      "ssn",
    ],
    censor: "[REDACTED]",
  },
});

function sanitizeHeaders(headers: Record<string, unknown>): Record<string, unknown> {
  const sensitiveKeys = [
    "authorization",
    "cookie",
    "x-api-key",
    "set-cookie",
    "x-session-id",
    "x-csrf-token",
  ];
  const sanitized: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(headers)) {
    const lowered = key.toLowerCase();
    sanitized[key] = sensitiveKeys.some((sk) => lowered.includes(sk))
      ? "[REDACTED]"
      : value;
  }
  return sanitized;
}

export function createRequestLogger(req: Parameters<typeof logger.info>[1] & { req: unknown }) {
  return logger.child({ req });
}

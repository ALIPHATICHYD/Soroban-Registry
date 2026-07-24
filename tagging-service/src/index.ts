import express from "express";
import tagRouter from "./controller.js";
import { startCronJobs } from "./cron.js";
import { createContractsRouter } from "./contracts.controller.js";
import { logger } from "./logger.js";
import { errorHandler, notFoundHandler } from "./middleware/errorHandler.js";

const app = express();
const PORT = parseInt(process.env.PORT || "3002", 10);

app.use(express.json());

app.use((req, _res, next) => {
  logger.info(
    {
      method: req.method,
      path: req.path,
      query: req.query,
      contentLength: req.headers["content-length"],
      contentType: req.headers["content-type"],
    },
    `${req.method} ${req.path}`,
  );
  next();
});

app.use(tagRouter);
app.use("/api/contracts", createContractsRouter());

app.get("/health", (_req, res) => {
  res.json({ status: "ok", service: "tagging-service" });
});

app.use(notFoundHandler);
app.use(errorHandler);

app.listen(PORT, () => {
  logger.info({ port: PORT }, `tagging-service running on port ${PORT}`);
  startCronJobs();
});

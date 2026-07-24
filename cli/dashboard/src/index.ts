#!/usr/bin/env node

import { Command } from "commander";
import { runDashboard } from "./dashboard/run";

type DashboardOptions = {
  refreshRate?: string;
  network?: string;
  category?: string;
};

const program = new Command();

program
  .name("soroban-registry")
  .description("Soroban-Registry CLI dashboard")
  .version("0.1.0");

program
  .command("dashboard")
  .description("Launch an interactive, real-time terminal dashboard")
  .option("--refresh-rate <ms>", "Minimum interval between UI renders", "100")
  .option("--network <name>", "Network filter (e.g. testnet, mainnet)")
  .option("--category <type>", "Contract category filter (e.g. dex, nft)")
  .action(async (opts: DashboardOptions) => {
    const refreshRateMs = Number.parseInt(opts.refreshRate ?? "100", 10);
    if (!Number.isFinite(refreshRateMs) || refreshRateMs <= 0) {
      process.stderr.write("--refresh-rate must be a positive integer (ms)\n");
      process.exitCode = 1;
      return;
    }

    await runDashboard({
      refreshRateMs,
      network: opts.network,
      category: opts.category,
    });
  });

program.parseAsync(process.argv).catch((err: unknown) => {
  const message = err instanceof Error ? err.message : String(err);
  process.stderr.write(`${message}\n`);
  process.exitCode = 1;
});

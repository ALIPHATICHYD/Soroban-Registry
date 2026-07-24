"use client";

import { useMemo, useState } from "react";
import {
  CreditCard,
  FileText,
  Receipt,
  ShieldCheck,
  ShoppingCart,
  Zap,
} from "lucide-react";
import Navbar from "@/components/Navbar";

type Product = {
  id: string;
  name: string;
  tier: string;
  price: number;
  network: string;
  summary: string;
};

const PRODUCTS: Product[] = [
  {
    id: "dex-pro",
    name: "DEX Routing Engine",
    tier: "Commercial",
    price: 249,
    network: "mainnet",
    summary:
      "Multi-hop routing logic with upgrade entitlement and enterprise support.",
  },
  {
    id: "vault-kit",
    name: "Vault Strategy Kit",
    tier: "Team",
    price: 149,
    network: "testnet",
    summary:
      "Managed strategy templates for yield vault contracts and staging deployments.",
  },
  {
    id: "oracle-feed",
    name: "Oracle Feed Adapter",
    tier: "Starter",
    price: 79,
    network: "futurenet",
    summary:
      "Lightweight price feed license with API examples and schema docs.",
  },
];

const TRANSACTIONS = [
  {
    id: "TX-1042",
    item: "DEX Routing Engine",
    amount: "$249",
    status: "Settled",
  },
  {
    id: "TX-1038",
    item: "Vault Strategy Kit",
    amount: "$149",
    status: "Pending",
  },
  {
    id: "TX-1027",
    item: "Oracle Feed Adapter",
    amount: "$79",
    status: "Settled",
  },
];

const LICENSES = [
  { name: "DEX Routing Engine", seats: "12 seats", renews: "2026-01-18" },
  { name: "Oracle Feed Adapter", seats: "3 seats", renews: "2025-11-02" },
];

export default function MarketplacePage() {
  const [cart, setCart] = useState<Product[]>([PRODUCTS[0]]);

  const subtotal = useMemo(
    () => cart.reduce((sum, item) => sum + item.price, 0),
    [cart],
  );

  const addToCart = (product: Product) => {
    setCart((current) => {
      if (current.some((item) => item.id === product.id)) {
        return current;
      }
      return [...current, product];
    });
  };

  const removeFromCart = (productId: string) => {
    setCart((current) => current.filter((item) => item.id !== productId));
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      <section className="hero-gradient border-b border-border">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-18 md:py-24">
          <div className="max-w-3xl">
            <div className="inline-flex items-center gap-2 rounded-full border border-primary/20 bg-background/75 px-4 py-2 text-sm font-medium text-primary backdrop-blur">
              <ShoppingCart className="h-4 w-4" />
              Marketplace scaffold
            </div>
            <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-5xl">
              Contract marketplace with cart and transaction flows
            </h1>
            <p className="mt-4 max-w-2xl text-lg text-muted-foreground">
              Mock UI for license sales, checkout review, payment capture,
              transaction history, and active license management.
            </p>
          </div>
        </div>
      </section>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-10 space-y-10">
        <section className="grid gap-6 lg:grid-cols-[1.6fr_1fr]">
          <div className="space-y-6">
            <div className="grid gap-5 md:grid-cols-2 xl:grid-cols-3">
              {PRODUCTS.map((product) => (
                <article
                  key={product.id}
                  className="gradient-border-card card-hover p-6"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="text-xs uppercase tracking-[0.25em] text-muted-foreground">
                        {product.network}
                      </p>
                      <h2 className="mt-2 text-xl font-semibold">
                        {product.name}
                      </h2>
                    </div>
                    <span className="rounded-full bg-primary/10 px-3 py-1 text-xs font-semibold text-primary">
                      {product.tier}
                    </span>
                  </div>
                  <p className="mt-4 text-sm leading-6 text-muted-foreground">
                    {product.summary}
                  </p>
                  <div className="mt-6 flex items-end justify-between">
                    <div>
                      <p className="text-xs text-muted-foreground">
                        License price
                      </p>
                      <p className="text-3xl font-bold">${product.price}</p>
                    </div>
                    <button
                      type="button"
                      onClick={() => addToCart(product)}
                      className="rounded-xl bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
                    >
                      Add to cart
                    </button>
                  </div>
                </article>
              ))}
            </div>

            <section className="rounded-3xl border border-border bg-card p-6 shadow-sm">
              <div className="flex items-center gap-3">
                <Receipt className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Transaction history</h2>
              </div>
              <div className="mt-5 space-y-3">
                {TRANSACTIONS.map((transaction) => (
                  <div
                    key={transaction.id}
                    className="flex flex-col gap-3 rounded-2xl border border-border/70 bg-background/70 p-4 md:flex-row md:items-center md:justify-between"
                  >
                    <div>
                      <p className="font-medium">{transaction.item}</p>
                      <p className="text-sm text-muted-foreground">
                        {transaction.id}
                      </p>
                    </div>
                    <div className="flex items-center gap-4">
                      <span className="text-sm font-semibold">
                        {transaction.amount}
                      </span>
                      <span className="rounded-full bg-accent px-3 py-1 text-xs font-medium text-accent-foreground">
                        {transaction.status}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </div>

          <div className="space-y-6">
            <section className="rounded-3xl border border-border bg-card p-6 shadow-sm">
              <div className="flex items-center gap-3">
                <ShoppingCart className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Cart</h2>
              </div>
              <div className="mt-5 space-y-3">
                {cart.length === 0 ? (
                  <p className="rounded-2xl border border-dashed border-border p-4 text-sm text-muted-foreground">
                    Cart is empty. Add a license to start checkout.
                  </p>
                ) : (
                  cart.map((item) => (
                    <div
                      key={item.id}
                      className="rounded-2xl border border-border/70 p-4"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="font-medium">{item.name}</p>
                          <p className="text-sm text-muted-foreground">
                            {item.tier} license
                          </p>
                        </div>
                        <button
                          type="button"
                          onClick={() => removeFromCart(item.id)}
                          className="text-sm text-muted-foreground hover:text-foreground"
                        >
                          Remove
                        </button>
                      </div>
                      <p className="mt-3 text-lg font-semibold">
                        ${item.price}
                      </p>
                    </div>
                  ))
                )}
              </div>
            </section>

            <section className="rounded-3xl border border-border bg-card p-6 shadow-sm">
              <div className="flex items-center gap-3">
                <CreditCard className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">Checkout</h2>
              </div>
              <div className="mt-5 space-y-4">
                <div className="rounded-2xl bg-accent/60 p-4 text-sm">
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Subtotal</span>
                    <span className="font-semibold">${subtotal}</span>
                  </div>
                  <div className="mt-2 flex items-center justify-between">
                    <span className="text-muted-foreground">Processing</span>
                    <span className="font-semibold">
                      ${cart.length === 0 ? 0 : 12}
                    </span>
                  </div>
                  <div className="mt-3 border-t border-border pt-3 flex items-center justify-between text-base">
                    <span className="font-semibold">Total</span>
                    <span className="font-bold">
                      ${subtotal + (cart.length === 0 ? 0 : 12)}
                    </span>
                  </div>
                </div>
                <button
                  type="button"
                  className="w-full rounded-2xl bg-primary px-4 py-3 font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={cart.length === 0}
                >
                  Complete mock checkout
                </button>
                <p className="text-xs text-muted-foreground">
                  Payment integration is intentionally stubbed. This page is for
                  flow and layout scaffolding.
                </p>
              </div>
            </section>

            <section className="rounded-3xl border border-border bg-card p-6 shadow-sm">
              <div className="flex items-center gap-3">
                <ShieldCheck className="h-5 w-5 text-primary" />
                <h2 className="text-xl font-semibold">License dashboard</h2>
              </div>
              <div className="mt-5 space-y-3">
                {LICENSES.map((license) => (
                  <div
                    key={license.name}
                    className="rounded-2xl border border-border/70 bg-background/70 p-4"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="font-medium">{license.name}</p>
                      <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-semibold text-emerald-600 dark:text-emerald-300">
                        Active
                      </span>
                    </div>
                    <div className="mt-3 flex items-center justify-between text-sm text-muted-foreground">
                      <span>{license.seats}</span>
                      <span>Renews {license.renews}</span>
                    </div>
                  </div>
                ))}
              </div>
            </section>
          </div>
        </section>

        <section className="grid gap-5 md:grid-cols-3">
          <div className="rounded-3xl border border-border bg-card p-6 shadow-sm">
            <Zap className="h-5 w-5 text-primary" />
            <h3 className="mt-4 text-lg font-semibold">Fast procurement</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Buyer flow is arranged around quick add-to-cart actions and
              compact checkout review.
            </p>
          </div>
          <div className="rounded-3xl border border-border bg-card p-6 shadow-sm">
            <FileText className="h-5 w-5 text-primary" />
            <h3 className="mt-4 text-lg font-semibold">License records</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              Dashboard cards reserve space for seat counts, renewal dates, and
              entitlement details.
            </p>
          </div>
          <div className="rounded-3xl border border-border bg-card p-6 shadow-sm">
            <Receipt className="h-5 w-5 text-primary" />
            <h3 className="mt-4 text-lg font-semibold">Transaction tracking</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              History rows are already separated from the cart flow so real
              settlement events can slot in later.
            </p>
          </div>
        </section>
      </main>
    </div>
  );
}

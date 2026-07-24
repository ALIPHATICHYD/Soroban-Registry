'use client';

import { useQuery } from '@tanstack/react-query';
import { api } from '@/lib/api';
import ContractCard from '@/components/ContractCard';
import ContractCardSkeleton from '@/components/ContractCardSkeleton';
import LoadingSkeleton from '@/components/LoadingSkeleton';
import { Search, Package, CheckCircle, Users, ArrowRight, Sparkles, Shield, GitBranch, Upload, Terminal, Github, MessageCircle, BookOpen, Zap } from 'lucide-react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import { useAnalytics } from '@/hooks/useAnalytics';
import { useTranslation } from '@/lib/i18n/client';
import Navbar from '@/components/Navbar';
import ActivityFeed from '@/components/ActivityFeed';
import { useCopy } from '@/hooks/useCopy';
import CodeCopyButton from '@/components/CodeCopyButton';

export default function Home() {
  const { t } = useTranslation('common');
  const router = useRouter();
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);
  const { logEvent } = useAnalytics();
  const { copy, copied, isCopying } = useCopy();

  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ['stats'],
    queryFn: () => api.getStats(),
  });

  const { data: recentContracts, isLoading: contractsLoading } = useQuery({
    queryKey: ['contracts', 'recent'],
    queryFn: () => api.getContracts({ page: 1, page_size: 6 }),
  });

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      logEvent('search_performed', {
        keyword: searchQuery.trim(),
        source: 'home_hero',
      });
      router.push(`/contracts?query=${encodeURIComponent(searchQuery)}`);
    }
  };

  const handleCopyCode = async () => {
    const code = `cargo install soroban-registry-cli\nsoroban-registry search token\nsoroban-registry install my-token-contract`;
    await copy(code, {
      successEventName: 'landing_cli_code_copied',
      failureEventName: 'landing_cli_code_copy_failed',
      successMessage: 'CLI example copied',
      failureMessage: 'Unable to copy CLI example',
      analyticsParams: { source: 'home_cli_block' },
    });
  };

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const isSlashShortcut = event.key === '/' || event.code === 'Slash';
      if (!isSlashShortcut || event.ctrlKey || event.metaKey || event.altKey) return;

      const activeElement = document.activeElement as HTMLElement | null;
      const isTypingField = Boolean(
        activeElement &&
        (activeElement.tagName === 'INPUT' ||
          activeElement.tagName === 'TEXTAREA' ||
          activeElement.tagName === 'SELECT' ||
          activeElement.isContentEditable),
      );

      if (isTypingField) return;

      event.preventDefault();
      searchInputRef.current?.focus();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div className="min-h-screen bg-background text-foreground">
      <Navbar />

      {/* Hero Section */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 bg-grid-pattern opacity-5 text-primary" />
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24 relative">
          <div className="text-center max-w-3xl mx-auto">
            <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-primary/10 text-primary text-sm font-medium mb-6">
              <Sparkles className="w-4 h-4" />
              The Official Soroban Smart Contract Registry
            </div>

            <h1 className="text-5xl sm:text-6xl font-bold mb-6 leading-tight">
              {t('home.title_part1')}
              <br />
              <span className="text-gradient">
                {t('home.title_part2')}
              </span>
            </h1>

            <p className="text-xl text-muted-foreground mb-12">
              {t('home.subtitle')}
            </p>

            {/* Search Bar */}
            <form onSubmit={handleSearch} className="max-w-2xl mx-auto mb-12">
              <div className="relative">
                <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-5 h-5 text-muted-foreground" />
                <input
                  ref={searchInputRef}
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search contracts by name, category, or tag..."
                  aria-label="Search contracts"
                  aria-keyshortcuts="/"
                  className="w-full pl-12 pr-4 py-4 rounded-xl border border-border bg-background text-foreground placeholder-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary shadow-lg"
                />
                <button
                  type="submit"
                  className="absolute right-2 top-1/2 -translate-y-1/2 px-6 py-2 rounded-lg bg-primary text-primary-foreground hover:opacity-90 transition-opacity font-medium"
                >
                  Search
                </button>
              </div>
            </form>

            {/* Stats */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 max-w-3xl mx-auto">
              {statsLoading ? (
                <>
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="bg-background rounded-xl p-6 border border-border shadow-sm">
                      <div className="flex items-center justify-center gap-2 mb-2">
                        <LoadingSkeleton width="3rem" height="2.25rem" />
                      </div>
                      <LoadingSkeleton width="7rem" height="0.875rem" className="mx-auto" />
                    </div>
                  ))}
                </>
              ) : stats ? (
                <>
                  <div className="bg-background rounded-xl p-6 border border-border shadow-sm">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <Package className="w-5 h-5 text-primary" />
                      <span className="text-3xl font-bold">
                        {stats.total_contracts}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground">Total Contracts</p>
                  </div>

                  <div className="bg-background rounded-xl p-6 border border-border shadow-sm">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <CheckCircle className="w-5 h-5 text-green-500" />
                      <span className="text-3xl font-bold">
                        {stats.verified_contracts}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground">Verified</p>
                  </div>

                  <div className="bg-background rounded-xl p-6 border border-border shadow-sm">
                    <div className="flex items-center justify-center gap-2 mb-2">
                      <Users className="w-5 h-5 text-secondary" />
                      <span className="text-3xl font-bold">
                        {stats.total_publishers}
                      </span>
                    </div>
                    <p className="text-sm text-muted-foreground">Publishers</p>
                  </div>
                </>
              ) : null}
            </div>
          </div>
        </div>
      </section>

      {/* Why Soroban Registry — Feature Cards */}
      <section className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold mb-4">
            {t('home.whyTitle')} <span className="text-gradient">Registry</span>
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            {t('home.whySubtitle')}
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          <div className="gradient-border-card p-8 card-hover">
            <div className="w-12 h-12 rounded-xl bg-green-500/10 flex items-center justify-center mb-6">
              <Shield className="w-6 h-6 text-green-500" />
            </div>
            <h3 className="text-xl font-semibold mb-3">{t('home.features.verified.title')}</h3>
            <p className="text-muted-foreground leading-relaxed">
              {t('home.features.verified.desc')}
            </p>
          </div>

          <div className="gradient-border-card p-8 card-hover">
            <div className="w-12 h-12 rounded-xl bg-primary/10 flex items-center justify-center mb-6">
              <GitBranch className="w-6 h-6 text-primary" />
            </div>
            <h3 className="text-xl font-semibold mb-3">{t('home.features.graph.title')}</h3>
            <p className="text-muted-foreground leading-relaxed">
              {t('home.features.graph.desc')}
            </p>
          </div>

          <div className="gradient-border-card p-8 card-hover">
            <div className="w-12 h-12 rounded-xl bg-secondary/10 flex items-center justify-center mb-6">
              <Upload className="w-6 h-6 text-secondary" />
            </div>
            <h3 className="text-xl font-semibold mb-3">{t('home.features.easy.title')}</h3>
            <p className="text-muted-foreground leading-relaxed">
              {t('home.features.easy.desc')}
            </p>
          </div>
        </div>
      </section>

      {/* Recent Contracts */}
      <section className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
        <div className="flex items-center justify-between mb-8">
          <h2 className="text-3xl font-bold">
            {t('home.recent')}
          </h2>
          <Link
            href="/contracts"
            className="flex items-center gap-2 text-primary hover:opacity-80 font-medium transition-opacity"
          >
            {t('home.viewAll')}
            <ArrowRight className="w-4 h-4" />
          </Link>
        </div>

        {contractsLoading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[1, 2, 3, 4, 5, 6].map((i) => (
              <ContractCardSkeleton key={i} />
            ))}
          </div>
        ) : recentContracts && (recentContracts.items?.length ?? 0) > 0 ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {(recentContracts.items ?? []).map((contract) => (
              <ContractCard key={contract.id} contract={contract} />
            ))}
          </div>
        ) : (
          <div className="text-center py-12 rounded-2xl border border-border bg-card">
            <Package className="w-12 h-12 text-muted-foreground mx-auto mb-4" />
            <p className="text-muted-foreground">No contracts published yet</p>
          </div>
        )}
      </section>

    {/* Activity Feed Section */}
    <section className="bg-muted/30 border-y border-border">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-12">
          <div className="lg:col-span-2">
            <ActivityFeed />
          </div>
          <div className="space-y-8">
            <div className="bg-card border border-border rounded-xl p-6 shadow-sm">
              <h3 className="text-lg font-bold mb-4 flex items-center gap-2">
                <Sparkles className="w-5 h-5 text-amber-500" />
                Live Insights
              </h3>
              <p className="text-sm text-muted-foreground mb-6">
                The registry is alive with activity. Watch as developers publish, verify, and deploy contracts in real-time.
              </p>
              <div className="space-y-4">
                <div className="flex items-start gap-3">
                  <div className="mt-1 p-1.5 rounded-full bg-blue-500/10 text-blue-500">
                    <Upload className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">Publishing</h4>
                    <p className="text-xs text-muted-foreground">New contracts added to the registry</p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <div className="mt-1 p-1.5 rounded-full bg-emerald-500/10 text-emerald-500">
                    <CheckCircle className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">Verification</h4>
                    <p className="text-xs text-muted-foreground">Source code validated by our nodes</p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <div className="mt-1 p-1.5 rounded-full bg-amber-500/10 text-amber-500">
                    <Zap className="w-4 h-4" />
                  </div>
                  <div>
                    <h4 className="text-sm font-semibold">Deployments</h4>
                    <p className="text-xs text-muted-foreground">Contracts going live on Stellar networks</p>
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-gradient-to-br from-primary/10 to-secondary/10 border border-primary/20 rounded-xl p-6 shadow-sm">
              <h3 className="text-lg font-bold mb-2">Build Together</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Share your contracts with the ecosystem and help other builders.
              </p>
              <Link 
                href="/publish"
                className="w-full py-2 bg-primary text-primary-foreground rounded-lg font-medium text-sm flex items-center justify-center gap-2"
              >
                Publish Your Contract
                <ArrowRight className="w-4 h-4" />
              </Link>
            </div>
          </div>
        </div>
      </div>
    </section>

      {/* Install & Learn — Code Section */}
      <section className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div>
            <h2 className="text-3xl sm:text-4xl font-bold mb-6">
              Install & start <span className="text-gradient">building</span>
            </h2>
            <p className="text-lg text-muted-foreground mb-8 leading-relaxed">
              Get up and running in minutes. Install the CLI, search the registry,
              and integrate verified contracts into your Soroban project.
            </p>
            <div className="flex flex-col sm:flex-row gap-4">
              <Link
                href="/contracts"
                className="btn-glow inline-flex items-center justify-center gap-2 px-6 py-3 rounded-xl bg-primary text-primary-foreground font-medium"
              >
                Browse Contracts
                <ArrowRight className="w-4 h-4" />
              </Link>
              <Link
                href="/templates"
                className="inline-flex items-center justify-center gap-2 px-6 py-3 rounded-xl border border-border text-foreground hover:bg-accent font-medium transition-all"
              >
                View Templates
              </Link>
            </div>
          </div>

          <div className="rounded-2xl overflow-hidden border border-border bg-[#0d1117]">
            <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
              <div className="flex items-center gap-2">
                <Terminal className="w-4 h-4 text-muted-foreground" />
                <span className="text-xs text-muted-foreground font-mono">Terminal</span>
              </div>
              <CodeCopyButton
                onCopy={handleCopyCode}
                copied={copied}
                disabled={isCopying}
                idleLabel="Copy"
                copiedLabel="Copied"
                className="border-white/10 bg-transparent text-gray-400 hover:bg-white/10 hover:text-white"
              />
            </div>
            <div className="p-6 font-mono text-sm leading-relaxed">
              <div className="text-gray-500 mb-1"># Install the CLI</div>
              <div className="text-green-400 mb-4">$ cargo install soroban-registry-cli</div>
              <div className="text-gray-500 mb-1"># Search for contracts</div>
              <div className="text-green-400 mb-4">$ soroban-registry search token</div>
              <div className="text-gray-500 mb-1"># Install a contract</div>
              <div className="text-green-400">$ soroban-registry install my-token-contract</div>
            </div>
          </div>
        </div>
      </section>

      {/* Community / CTA Section */}
      <section className="border-t border-border bg-accent/50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-24">
          <div className="text-center mb-12">
            <h2 className="text-3xl sm:text-4xl font-bold mb-4">
              Join the <span className="text-gradient">community</span>
            </h2>
            <p className="text-lg text-muted-foreground max-w-xl mx-auto">
              Connect with developers building the future of DeFi on Stellar.
            </p>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-6 max-w-3xl mx-auto">
            <a
              href="https://github.com/stellar"
              target="_blank"
              rel="noreferrer"
              className="gradient-border-card p-6 text-center card-hover group"
            >
              <div className="w-14 h-14 rounded-2xl bg-card flex items-center justify-center mx-auto mb-4 border border-border group-hover:border-primary/50 transition-colors">
                <Github className="w-7 h-7 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>
              <h3 className="font-semibold mb-1">GitHub</h3>
              <p className="text-sm text-muted-foreground">Contribute to the codebase</p>
            </a>

            <a
              href="https://discord.com/invite/stellardev"
              target="_blank"
              rel="noreferrer"
              className="gradient-border-card p-6 text-center card-hover group"
            >
              <div className="w-14 h-14 rounded-2xl bg-card flex items-center justify-center mx-auto mb-4 border border-border group-hover:border-primary/50 transition-colors">
                <MessageCircle className="w-7 h-7 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>
              <h3 className="font-semibold mb-1">Discord</h3>
              <p className="text-sm text-muted-foreground">Chat with developers</p>
            </a>

            <a
              href="https://developers.stellar.org/docs/smart-contracts"
              target="_blank"
              rel="noreferrer"
              className="gradient-border-card p-6 text-center card-hover group"
            >
              <div className="w-14 h-14 rounded-2xl bg-card flex items-center justify-center mx-auto mb-4 border border-border group-hover:border-primary/50 transition-colors">
                <BookOpen className="w-7 h-7 text-muted-foreground group-hover:text-primary transition-colors" />
              </div>
              <h3 className="font-semibold mb-1">Documentation</h3>
              <p className="text-sm text-muted-foreground">Read the Soroban docs</p>
            </a>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border bg-card" aria-label="Site footer">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-16">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8 mb-12">
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.registry')}</h4>
              <ul className="space-y-3 text-sm">
                <li><Link href="/contracts" className="text-muted-foreground hover:text-foreground transition-colors">{t('home.viewContracts')}</Link></li>
                <li><Link href="/templates" className="text-muted-foreground hover:text-foreground transition-colors">Templates</Link></li>
                <li><Link href="/publish" className="text-muted-foreground hover:text-foreground transition-colors">Publish</Link></li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.explore')}</h4>
              <ul className="space-y-3 text-sm">
                <li><Link href="/graph" className="text-muted-foreground hover:text-foreground transition-colors">Dependency Graph</Link></li>
                <li><Link href="/stats" className="text-muted-foreground hover:text-foreground transition-colors">Statistics</Link></li>
                <li><Link href="/publishers" className="text-muted-foreground hover:text-foreground transition-colors">Publishers</Link></li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.developers')}</h4>
              <ul className="space-y-3 text-sm">
                <li><a href="https://developers.stellar.org/docs/smart-contracts" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Soroban Docs</a></li>
                <li><a href="https://stellar.org/soroban" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">About Soroban</a></li>
                <li><a href="https://github.com/stellar" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">GitHub</a></li>
              </ul>
            </div>
            <div>
              <h4 className="font-semibold text-foreground mb-4 text-sm uppercase tracking-wider">{t('footer.community')}</h4>
              <ul className="space-y-3 text-sm">
                <li><a href="https://discord.com/invite/stellardev" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Discord</a></li>
                <li><a href="https://twitter.com/BuildOnStellar" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Twitter</a></li>
                <li><a href="https://stellar.org/community" target="_blank" rel="noreferrer" className="text-muted-foreground hover:text-foreground transition-colors">Stellar Community</a></li>
              </ul>
            </div>
          </div>

          <div className="border-t border-border pt-8 flex flex-col sm:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-2 text-muted-foreground text-sm">
              <Package className="w-4 h-4 text-primary" />
              <span>{t('footer.builtFor')}</span>
            </div>
            <p className="text-sm text-muted-foreground">{t('footer.poweredBy')}</p>
          </div>
        </div>
      </footer>
    </div>
  );
}

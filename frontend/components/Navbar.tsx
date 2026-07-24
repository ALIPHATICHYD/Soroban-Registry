'use client';

import { Package, GitBranch, ChevronDown, BarChart2, Users, Menu, X, Layers, Search, Plus, Columns2, ShieldCheck, PieChart, TrendingUp, LogOut, Settings, Zap, Code2, User, ShoppingCart, Star } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import React, { useState, useRef, useEffect, useCallback } from 'react';
import ThemeToggle from './ThemeToggle';
import NotificationBell from './NotificationBell';
import { useTranslation } from '@/lib/i18n/client';
import LanguageSelector from './LanguageSelector';
import { useFavorites } from '@/hooks/useFavorites';

/* ─── nav links ──────────────────────────────────────────── */
const NAV_LINKS = [
    { href: '/contracts',       label: 'Browse',  icon: Package   },
    { href: '/compare',         label: 'Compare', icon: Columns2  },
    { href: '/marketplace',     label: 'Market',  icon: ShoppingCart },
    { href: '/verify-contract', label: 'Verify',  icon: ShieldCheck },
    { href: '/developer',       label: 'IDE',     icon: Code2 },
] as const;

const EXPLORE_LINKS = [
    { href: '/publishers', label: 'Publishers', icon: Users     },
    { href: '/stats',      label: 'Statistics', icon: BarChart2 },
    { href: '/analytics',  label: 'Analytics',  icon: PieChart  },
    { href: '/templates',  label: 'Templates',  icon: Layers    },
] as const;

/* ─── helpers ─────────────────────────────────────────────── */
function useScrolled(threshold = 8) {
    const [scrolled, setScrolled] = useState(false);
    useEffect(() => {
        const onScroll = () => setScrolled(window.scrollY > threshold);
        window.addEventListener('scroll', onScroll, { passive: true });
        return () => window.removeEventListener('scroll', onScroll);
    }, [threshold]);
    return scrolled;
}

function useTrapFocus(ref: React.RefObject<HTMLElement | null>, active: boolean) {
    useEffect(() => {
        if (!active || !ref.current) return;
        const el = ref.current;
        const focusable = el.querySelectorAll<HTMLElement>(
            'a[href], button:not([disabled]), input, [tabindex]:not([tabindex="-1"])'
        );
        const first = focusable[0];
        const last  = focusable[focusable.length - 1];
        const onKey = (e: KeyboardEvent) => {
            if (e.key !== 'Tab') return;
            if (e.shiftKey ? document.activeElement === first : document.activeElement === last) {
                e.preventDefault();
                (e.shiftKey ? last : first).focus();
            }
        };
        el.addEventListener('keydown', onKey);
        first?.focus();
        return () => el.removeEventListener('keydown', onKey);
    }, [active, ref]);
}

// Keep useTrapFocus callable but suppress the unused-vars warning via
// a no-op reference so tree-shaking doesn't affect the export.
void (useTrapFocus as unknown);

/* ─── Search Modal ─────────────────────────────────────────── */
function SearchModal({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
    const inputRef = useRef<HTMLInputElement>(null);
    const [query, setQuery] = useState('');

    useEffect(() => {
        if (isOpen) {
            setTimeout(() => {
                inputRef.current?.focus();
                setQuery('');
            }, 0);
        }
    }, [isOpen]);

    useEffect(() => {
        const onKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') onClose();
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); }
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, [onClose]);

    const suggestions = [
        { label: 'Token Contract',     href: '/contracts?q=token',   icon: Zap    },
        { label: 'NFT Marketplace',    href: '/contracts?q=nft',     icon: Layers },
        { label: 'DeFi Protocols',     href: '/contracts?q=defi',    icon: TrendingUp},
        { label: 'DAO Governance',     href: '/contracts?q=dao',     icon: Users  },
        { label: 'Smart Contract SDK', href: '/contracts?q=sdk',     icon: Code2  },
    ];

    return (
        <div
            className={`fixed inset-0 z-[200] flex items-start justify-center pt-16 sm:pt-24 px-4 transition-all duration-200 ${
                isOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'
            }`}
            role="dialog"
            aria-modal="true"
            aria-label="Search"
        >
            {/* Backdrop */}
            <div
                className="absolute inset-0 bg-background/70 backdrop-blur-md"
                onClick={onClose}
            />

            {/* Modal panel */}
            <div
                className={`relative w-full max-w-lg bg-card border border-border rounded-2xl shadow-2xl overflow-hidden transition-all duration-200 ${
                    isOpen ? 'translate-y-0 scale-100' : '-translate-y-4 scale-95'
                }`}
            >
                {/* Search Input */}
                <div className="flex items-center gap-3 px-4 py-3 border-b border-border">
                    <Search className="w-5 h-5 text-muted-foreground flex-shrink-0" />
                    <input
                        ref={inputRef}
                        type="search"
                        value={query}
                        onChange={e => setQuery(e.target.value)}
                        placeholder="Search contracts, publishers, templates…"
                        className="flex-1 bg-transparent text-foreground text-sm focus:outline-none placeholder:text-muted-foreground"
                    />
                    <button
                        onClick={onClose}
                        className="flex items-center gap-1 px-2 py-1 rounded-md border border-border text-xs text-muted-foreground hover:border-primary/50 transition-colors"
                        aria-label="Close search"
                    >
                        <span>ESC</span>
                    </button>
                </div>

                {/* Suggestions */}
                <div className="p-2">
                    <p className="px-3 py-1.5 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">
                        {query ? 'Results' : 'Popular searches'}
                    </p>
                    <ul>
                        {suggestions.map(({ label, href, icon: Icon }) => (
                            <li key={href}>
                                <Link
                                    href={`${href}${query ? `&q=${encodeURIComponent(query)}` : ''}`}
                                    onClick={onClose}
                                    className="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-accent transition-colors group"
                                >
                                    <span className="w-7 h-7 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0 group-hover:bg-primary/20 transition-colors">
                                        <Icon className="w-3.5 h-3.5 text-primary" />
                                    </span>
                                    {label}
                                </Link>
                            </li>
                        ))}
                    </ul>
                </div>

                {/* Footer */}
                <div className="px-4 py-2.5 border-t border-border bg-accent/30 flex items-center justify-between">
                    <span className="text-xs text-muted-foreground">
                        Press <kbd className="px-1.5 py-0.5 rounded border border-border bg-card text-foreground font-mono text-[10px]">↑↓</kbd> to navigate
                    </span>
                    <Link href={`/contracts${query ? `?q=${encodeURIComponent(query)}` : ''}`} onClick={onClose} className="text-xs text-primary hover:underline">
                        Browse all →
                    </Link>
                </div>
            </div>
        </div>
    );
}

/* ─── Navbar ────────────────────────────────────────────────── */
export default function Navbar() {
    const { t, i18n } = useTranslation('en');
    const lng = i18n.resolvedLanguage || 'en';
    const pathname = usePathname() ?? '';
    const scrolled = useScrolled();
    const { favoritesCount } = useFavorites();

    const [mobileOpen,    setMobileOpen]    = useState(false);
    const [exploreOpen,   setExploreOpen]   = useState(false);
    const [, setProfileOpen]   = useState(false);
    const [searchOpen,    setSearchOpen]    = useState(false);

    const exploreTimeout = useRef<NodeJS.Timeout | null>(null);
    const profileTimeout = useRef<NodeJS.Timeout | null>(null);
    const drawerRef      = useRef<HTMLDivElement>(null);

    // Close mobile menu on route change
    useEffect(() => { setTimeout(() => setMobileOpen(false), 0); }, [pathname]);

    // Prevent body scroll when mobile drawer open
    useEffect(() => {
        document.body.style.overflow = mobileOpen ? 'hidden' : '';
        return () => { document.body.style.overflow = ''; };
    }, [mobileOpen]);

    // ESC closes mobile drawer
    useEffect(() => {
        const onKey = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setMobileOpen(false);
            if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
                e.preventDefault();
                setSearchOpen(true);
            }
        };
        window.addEventListener('keydown', onKey);
        return () => window.removeEventListener('keydown', onKey);
    }, []);

    const isActive = useCallback((href: string) => pathname === href, [pathname]);
    const isExploreActive = EXPLORE_LINKS.some(l => pathname.startsWith(l.href));

    /* hover helpers */
    const onExploreEnter = () => { if (exploreTimeout.current) clearTimeout(exploreTimeout.current); setExploreOpen(true);  };
    const onExploreLeave = () => { exploreTimeout.current = setTimeout(() => setExploreOpen(false), 150); };
    const onProfileEnter = () => { if (profileTimeout.current) clearTimeout(profileTimeout.current); setProfileOpen(true); };
    const onProfileLeave = () => { profileTimeout.current = setTimeout(() => setProfileOpen(false), 150); };

    return (
        <>
            {/* ── Main nav bar ───────────────────────────────────────── */}
            <nav
                className={`sticky top-0 z-50 w-full transition-all duration-300 ${
                    scrolled
                        ? 'bg-background/95 backdrop-blur-2xl border-b border-border shadow-lg shadow-black/8'
                        : 'bg-background/80 backdrop-blur-2xl border-b border-border/50 nav-glow'
                }`}
                aria-label="Main navigation"
            >
                <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                    <div className="flex items-center justify-between h-14">

                        {/* Logo */}
                        <Link href="/" className="flex items-center gap-2 group flex-shrink-0" aria-label="Soroban Registry home">
                            <div className="w-7 h-7 rounded-md bg-gradient-to-br from-primary to-secondary flex items-center justify-center shadow-sm shadow-primary/25 group-hover:shadow-primary/50 transition-shadow">
                                <Package className="w-4 h-4 text-white" />
                            </div>
                            <span className="text-base font-bold text-foreground tracking-tight hidden sm:block">
                                Soroban<span className="text-primary">Registry</span>
                            </span>
                        </Link>

                        {/* ── Desktop nav links ─────────────────────────── */}
                        <div className="hidden md:flex items-center gap-0.5" role="menubar" aria-label="Site navigation">

                            {NAV_LINKS.map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    role="menuitem"
                                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[13px] font-medium transition-all ${
                                        isActive(href)
                                            ? 'text-primary bg-primary/10'
                                            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                    }`}
                                    aria-current={isActive(href) ? 'page' : undefined}
                                >
                                    <Icon className="w-3 h-3" />
                                    {t(`navbar.${label.toLowerCase()}`, label)}
                                </Link>
                            ))}

                            {/* Explore dropdown */}
                            <div
                                className="relative"
                                onMouseEnter={onExploreEnter}
                                onMouseLeave={onExploreLeave}
                                role="none"
                            >
                                <button
                                    role="menuitem"
                                    aria-haspopup="true"
                                    aria-expanded={exploreOpen}
                                    className={`flex items-center gap-1 px-3 py-1.5 rounded-md text-[13px] font-medium transition-all focus:outline-none ${
                                        isExploreActive || exploreOpen
                                            ? 'text-primary bg-primary/10'
                                            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                    }`}
                                >
                                    Explore
                                    <ChevronDown className={`w-3 h-3 transition-transform duration-200 ${exploreOpen ? 'rotate-180' : ''}`} />
                                </button>

                                <div
                                    role="menu"
                                    className={`absolute top-full left-1/2 -translate-x-1/2 w-48 pt-1.5 transition-all duration-150 ${
                                        exploreOpen
                                            ? 'opacity-100 translate-y-0 pointer-events-auto'
                                            : 'opacity-0 -translate-y-2 pointer-events-none'
                                    }`}
                                >
                                    <div className="rounded-xl border border-border bg-card shadow-xl shadow-black/12 overflow-hidden">
                                        <div className="py-1.5">
                                            {EXPLORE_LINKS.map(({ href, label, icon: Icon }) => (
                                                <Link
                                                    key={href}
                                                    href={href}
                                                    role="menuitem"
                                                    className={`flex items-center gap-2.5 px-3 py-2 text-[13px] transition-colors ${
                                                        isActive(href)
                                                            ? 'text-primary bg-primary/8'
                                                            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                                    }`}
                                                    aria-current={isActive(href) ? 'page' : undefined}
                                                >
                                                    <Icon className="w-3.5 h-3.5 text-primary/70" />
                                                    {label}
                                                </Link>
                                            ))}
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <Link
                                href="/graph"
                                role="menuitem"
                                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[13px] font-medium transition-all ${
                                    isActive('/graph')
                                        ? 'text-primary bg-primary/10'
                                        : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                }`}
                                aria-current={isActive('/graph') ? 'page' : undefined}
                            >
                                <GitBranch className="w-3 h-3" />
                                Graph
            </Link>
                        </div>

                        {/* ── Desktop right actions ────────────────────── */}
                        <div className="hidden md:flex items-center gap-1.5">
                            {/* Search bar */}
                            <button
                                onClick={() => setSearchOpen(true)}
                                className="hidden lg:flex items-center gap-2 h-8 w-44 px-3 rounded-md border border-border bg-background hover:bg-accent text-[13px] text-muted-foreground transition-all hover:border-primary/40 mr-1 group"
                                aria-label="Open search (⌘K)"
                            >
                                <Search className="w-3.5 h-3.5 flex-shrink-0" />
                                <span className="flex-1 text-left">Search…</span>
                                <kbd className="hidden xl:block px-1 py-0.5 rounded border border-border bg-accent text-[10px] font-mono text-muted-foreground group-hover:border-primary/30 transition-colors">⌘K</kbd>
                            </button>

                            <LanguageSelector lng={lng} />
                            <ThemeToggle />
                            <NotificationBell />

                            {/* Favorites link */}
                            <Link
                                href="/favorites"
                                aria-label="Your favorites"
                                className={`relative p-1.5 rounded-md transition-colors ${
                                    isActive('/favorites')
                                        ? 'text-primary bg-primary/10'
                                        : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                }`}
                            >
                                <Star className="w-5 h-5" />
                                {favoritesCount > 0 && (
                                    <span className="absolute top-0.5 right-0.5 flex items-center justify-center min-w-[1rem] h-4 px-0.5 text-[10px] font-bold text-primary-foreground bg-primary rounded-full">
                                        {favoritesCount > 99 ? '99+' : favoritesCount}
                                    </span>
                                )}
                            </Link>

                            {/* Profile dropdown */}
                            <div
                                className="relative ml-0.5"
                                onMouseEnter={onProfileEnter}
                                onMouseLeave={onProfileLeave}
                            >
                                <div className="rounded-lg border border-border bg-card shadow-lg shadow-black/8 overflow-hidden">
                                    <div className="py-1">
                                        <Link
                                            href="/publishers"
                                            className={`flex items-center gap-2.5 px-3 py-2 text-[13px] transition-colors ${
                                                isActive('/publishers')
                                                    ? 'text-primary bg-primary/5'
                                                    : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                            }`}
                                        >
                                            <Users className="w-3.5 h-3.5 text-primary/70" />
                                            Publishers
                                        </Link>
                                        <Link
                                            href="/stats"
                                            className={`flex items-center gap-2.5 px-3 py-2 text-[13px] transition-colors ${
                                                isActive('/stats')
                                                    ? 'text-primary bg-primary/5'
                                                    : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                            }`}
                                        >
                                            <BarChart2 className="w-3.5 h-3.5 text-primary/70" />
                                            Statistics
                                        </Link>
                                        <Link
                                            href="/analytics"
                                            className={`flex items-center gap-2.5 px-3 py-2 text-[13px] transition-colors ${
                                                isActive('/analytics')
                                                    ? 'text-primary bg-primary/5'
                                                    : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                            }`}
                                        >
                                            <PieChart className="w-3.5 h-3.5 text-primary/70" />
                                            Analytics
                                        </Link>
                                        <Link
                                            href="/templates"
                                            className={`flex items-center gap-2.5 px-3 py-2 text-[13px] transition-colors ${
                                                isActive('/templates')
                                                    ? 'text-primary bg-primary/5'
                                                    : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                            }`}
                                        >
                                            <Layers className="w-3.5 h-3.5 text-primary/70" />
                                            Templates
                                        </Link>
                                    </div>
                                </div>
                            </div>

                            <Link
                                href="/publish"
                                className="flex items-center gap-1.5 px-3.5 py-1.5 ml-1 rounded-md bg-primary text-primary-foreground text-[13px] font-semibold btn-glow transition-all hover:brightness-110 focus:outline-none focus:ring-2 focus:ring-primary/50"
                            >
                                <Plus className="w-3.5 h-3.5" />
                                Publish
                            </Link>
                        </div>

                        {/* ── Mobile actions row ───────────────────────── */}
                        <div className="flex md:hidden items-center gap-1">
                            {/* Mobile search button */}
                            <button
                                onClick={() => setSearchOpen(true)}
                                className="p-2 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                                aria-label="Open search"
                            >
                                <Search className="w-5 h-5" />
                            </button>

                            <LanguageSelector lng={lng} />
                            <ThemeToggle />

                            {/* Hamburger / close */}
                            <button
                                onClick={() => setMobileOpen(v => !v)}
                                className="p-2 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors focus:outline-none focus:ring-2 focus:ring-primary/50"
                                aria-label={mobileOpen ? 'Close menu' : 'Open menu'}
                                aria-expanded={mobileOpen}
                                aria-controls="mobile-nav-drawer"
                            >
                                <span className="relative w-5 h-5 flex items-center justify-center">
                                    <Menu
                                        className={`w-5 h-5 absolute transition-all duration-200 ${
                                            mobileOpen ? 'opacity-0 rotate-90 scale-75' : 'opacity-100 rotate-0 scale-100'
                                        }`}
                                    />
                                    <X
                                        className={`w-5 h-5 absolute transition-all duration-200 ${
                                            mobileOpen ? 'opacity-100 rotate-0 scale-100' : 'opacity-0 -rotate-90 scale-75'
                                        }`}
                                    />
                                </span>
                            </button>
                        </div>
                    </div>
                </div>
            </nav>

            {/* ── Mobile drawer overlay + panel ──────────────────────── */}
            <div
                id="mobile-nav-drawer"
                className={`fixed inset-0 z-[100] md:hidden transition-all duration-300 ${
                    mobileOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'
                }`}
                aria-hidden={!mobileOpen}
            >
                {/* Backdrop */}
                <div
                    className="absolute inset-0 bg-background/60 backdrop-blur-sm"
                    onClick={() => setMobileOpen(false)}
                    aria-hidden="true"
                />

                {/* Slide-out panel */}
                <div
                    ref={drawerRef}
                    role="dialog"
                    aria-modal="true"
                    aria-label="Mobile navigation menu"
                    className={`absolute inset-y-0 right-0 w-[80vw] max-w-sm flex flex-col bg-card border-l border-border shadow-2xl transition-transform duration-300 ease-in-out ${
                        mobileOpen ? 'translate-x-0' : 'translate-x-full'
                    }`}
                >
                    {/* Drawer header */}
                    <div className="flex items-center justify-between px-5 py-4 border-b border-border">
                        <Link href="/" className="flex items-center gap-2" onClick={() => setMobileOpen(false)}>
                            <div className="w-6 h-6 rounded-md bg-gradient-to-br from-primary to-secondary flex items-center justify-center">
                                <Package className="w-3.5 h-3.5 text-white" />
                            </div>
                            <span className="font-bold text-base text-foreground tracking-tight">
                                Soroban<span className="text-primary">Registry</span>
                            </span>
                        </Link>
                        <button
                            onClick={() => setMobileOpen(false)}
                            className="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors focus:outline-none"
                            aria-label="Close menu"
                        >
                            <X className="w-5 h-5" />
                        </button>
                    </div>

                    {/* Quick links section */}
                    <div className="px-3 pt-3 pb-2">
                        <p className="px-2 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Quick Links</p>
                        <div className="grid grid-cols-2 gap-1">
                            {[
                                { href: '/contracts', label: 'Browse Contracts', icon: Search },
                                { href: '/compare', label: 'Compare Contracts', icon: Columns2 },
                                { href: '/verify-contract', label: 'Verify Contract', icon: ShieldCheck },
                                { href: '/publishers', label: 'Publishers', icon: Users },
                                { href: '/stats', label: 'Statistics', icon: BarChart2 },
                                { href: '/analytics', label: 'Analytics', icon: PieChart },
                                { href: '/templates', label: 'Templates', icon: Layers },
                                { href: '/analytics', label: 'Search Analytics', icon: TrendingUp },
                                { href: '/graph', label: 'Dependency Graph', icon: GitBranch },
                                { href: '/developer', label: 'Contract IDE', icon: Code2 },
                            ].map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={`${href}-${label}`}
                                    href={href}
                                    onClick={() => setMobileOpen(false)}
                                    className={`flex items-center gap-2 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
                                        isActive(href)
                                            ? 'text-primary bg-primary/12 border border-primary/20'
                                            : 'text-muted-foreground hover:text-foreground hover:bg-accent border border-transparent'
                                    }`}
                                >
                                    <Icon className="w-4 h-4" />
                                    {label}
                                </Link>
                            ))}
                        </div>
                    </div>

                    {/* Divider */}
                    <div className="mx-3 border-t border-border" />

                    {/* All nav links */}
                    {/* Navigation links */}
                    <div className="flex-1 overflow-y-auto py-3 px-3">
                        <p className="px-2 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Navigation</p>
                        <nav className="flex flex-col gap-0.5" aria-label="Mobile navigation links">
                            {[
                                { href: '/contracts',       label: 'Browse Contracts',  icon: Package    },
                                { href: '/compare',         label: 'Compare Contracts', icon: Columns2   },
                                { href: '/verify-contract', label: 'Verify Contract',   icon: ShieldCheck},
                                { href: '/stats',           label: 'Statistics',        icon: BarChart2  },
                                { href: '/analytics',       label: 'Analytics',         icon: PieChart   },
                                { href: '/templates',       label: 'Templates',         icon: Layers     },
                                { href: '/graph',           label: 'Dependency Graph',  icon: GitBranch  },
                                { href: '/favorites',       label: 'My Favorites',      icon: Star       },
                            ].map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    onClick={() => setMobileOpen(false)}
                                    className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
                                        pathname === href
                                            ? 'text-primary bg-primary/10'
                                            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                    }`}
                                    aria-current={pathname === href ? 'page' : undefined}
                                >
                                    <span className={`w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors ${
                                        pathname === href ? 'bg-primary/20' : 'bg-accent'
                                    }`}>
                                        <Icon className={`w-3.5 h-3.5 ${pathname === href ? 'text-primary' : 'text-muted-foreground'}`} />
                                    </span>
                                    {label}
                                    {pathname === href && (
                                        <span className="ml-auto w-1.5 h-1.5 rounded-full bg-primary" />
                                    )}
                                </Link>
                            ))}
                        </nav>
                    </div>

                    {/* Profile footer */}
                    <div className="border-t border-border p-4 bg-accent/20">
                        <div className="flex items-center gap-3 mb-3">
                            <div className="w-10 h-10 rounded-full bg-gradient-to-br from-primary/20 to-secondary/20 border border-primary/20 flex items-center justify-center flex-shrink-0">
                                <User className="w-5 h-5 text-primary" />
                            </div>
                            <div className="flex-1 min-w-0">
                                <p className="text-sm font-semibold text-foreground truncate">User Profile</p>
                                <p className="text-xs text-muted-foreground truncate">user@example.com</p>
                            </div>
                            <Link
                                href="/settings"
                                onClick={() => setMobileOpen(false)}
                                className="p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                                aria-label="Settings"
                            >
                                <Settings className="w-4 h-4" />
                            </Link>
                        </div>

                        <div className="grid grid-cols-2 gap-2">
                            <Link
                                href="/publish"
                                onClick={() => setMobileOpen(false)}
                                className="flex items-center justify-center gap-1.5 py-2.5 rounded-lg bg-primary text-primary-foreground font-semibold text-sm btn-glow"
                            >
                                <Plus className="w-4 h-4" />
                                Publish
                            </Link>
                            <button
                                className="flex items-center justify-center gap-1.5 py-2.5 rounded-lg border border-red-500/20 text-red-500 text-sm font-medium hover:bg-red-500/8 transition-colors"
                                onClick={() => setMobileOpen(false)}
                            >
                                <LogOut className="w-4 h-4" />
                                Sign Out
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            {/* ── Global search modal ────────────────────────────────── */}
            <SearchModal isOpen={searchOpen} onClose={() => setSearchOpen(false)} />
        </>
    );
}

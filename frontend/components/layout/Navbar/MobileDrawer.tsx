'use client';

import Link from 'next/link';
import { 
    Package, X, Search, Columns2, ShieldCheck, Users, 
    BarChart2, PieChart, Layers, GitBranch, Code2, Star,
    User, Settings, Plus, LogOut, Home, ChevronRight
} from 'lucide-react';
import styles from './Navbar.module.css';

interface MobileDrawerProps {
    isOpen: boolean;
    onClose: () => void;
    pathname: string;
    favoritesCount: number;
    isActive: (href: string) => boolean;
}

const formatBreadcrumbLabel = (segment: string) =>
    decodeURIComponent(segment)
        .replace(/[-_]/g, ' ')
        .replace(/\b\w/g, (c) => c.toUpperCase());

export function MobileDrawer({ isOpen, onClose, pathname, favoritesCount, isActive }: MobileDrawerProps) {
    const mobileCrumbs = pathname.split('/').filter(Boolean).slice(0, 3);

    return (
        <div
            className={`${styles.drawer} ${isOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'}`}
            aria-hidden={!isOpen}
        >
            <div className="absolute inset-0 bg-background/60 backdrop-blur-sm" onClick={onClose} aria-hidden="true" />

            <div className={`${styles.drawerPanel} ${isOpen ? 'translate-x-0' : 'translate-x-full'}`} role="dialog" aria-modal="true">
                <div className="flex items-center justify-between px-5 py-4 border-b border-border">
                    <Link href="/" className="flex items-center gap-2" onClick={onClose}>
                        <div className="w-6 h-6 rounded-md bg-gradient-to-br from-primary to-secondary flex items-center justify-center">
                            <Package className="w-3.5 h-3.5 text-white" />
                        </div>
                        <span className="font-bold text-base text-foreground tracking-tight">
                            Soroban<span className="text-primary">Registry</span>
                        </span>
                    </Link>
                    <button onClick={onClose} className="p-1.5 rounded-md text-muted-foreground hover:text-foreground transition-colors">
                        <X className="w-5 h-5" />
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto">
                    <div className="px-3 pt-3 pb-2">
                        <p className="px-2 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Quick Links</p>
                        <div className="grid grid-cols-2 gap-1">
                            {[
                                { href: '/contracts', label: 'Browse', icon: Search },
                                { href: '/compare', label: 'Compare', icon: Columns2 },
                                { href: '/verify-contract', label: 'Verify', icon: ShieldCheck },
                                { href: '/publishers', label: 'Publishers', icon: Users },
                                { href: '/stats', label: 'Stats', icon: BarChart2 },
                                { href: '/analytics', label: 'Analytics', icon: PieChart },
                                { href: '/templates', label: 'Templates', icon: Layers },
                                { href: '/graph', label: 'Graph', icon: GitBranch },
                                { href: '/developer', label: 'IDE', icon: Code2 },
                            ].map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    onClick={onClose}
                                    className={`flex items-center gap-2 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
                                        isActive(href)
                                            ? 'text-primary bg-primary/12 border border-primary/20'
                                            : 'text-muted-foreground hover:bg-accent border border-transparent'
                                        }`}
                                >
                                    <Icon className="w-4 h-4" />
                                    {label}
                                </Link>
                            ))}
                        </div>
                    </div>

                    <div className="mx-3 border-t border-border" />

                    <div className="py-3 px-3">
                        <p className="px-2 pb-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider">Navigation</p>
                        <nav className="flex flex-col gap-0.5">
                            {[
                                { href: '/favorites', label: 'My Favorites', icon: Star, count: favoritesCount },
                            ].map(({ href, label, icon: Icon, count }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    onClick={onClose}
                                    className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
                                        pathname === href ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:bg-accent'
                                    }`}
                                >
                                    <span className={`w-7 h-7 rounded-lg flex items-center justify-center ${
                                        pathname === href ? 'bg-primary/20' : 'bg-accent'
                                    }`}>
                                        <Icon className={`w-3.5 h-3.5 ${pathname === href ? 'text-primary' : ''}`} />
                                    </span>
                                    {label}
                                    {count > 0 && <span className="ml-auto text-xs font-bold px-2 py-0.5 rounded-full bg-primary/10 text-primary">{count}</span>}
                                </Link>
                            ))}
                        </nav>
                    </div>
                </div>

                <div className="border-t border-border p-4 bg-accent/20">
                    <div className="flex items-center gap-3 mb-3">
                        <div className="w-10 h-10 rounded-full bg-gradient-to-br from-primary/20 to-secondary/20 border border-primary/20 flex items-center justify-center">
                            <User className="w-5 h-5 text-primary" />
                        </div>
                        <div className="flex-1 min-w-0">
                            <p className="text-sm font-semibold text-foreground truncate">User Profile</p>
                            <p className="text-xs text-muted-foreground truncate">user@example.com</p>
                        </div>
                        <Link href="/settings" onClick={onClose} className="p-1.5 rounded-md text-muted-foreground hover:bg-accent transition-colors">
                            <Settings className="w-4 h-4" />
                        </Link>
                    </div>
                    <div className="grid grid-cols-2 gap-2">
                        <Link href="/publish" onClick={onClose} className="flex items-center justify-center gap-1.5 py-2.5 rounded-lg bg-primary text-primary-foreground font-semibold text-sm">
                            <Plus className="w-4 h-4" /> Publish
                        </Link>
                        <button className="flex items-center justify-center gap-1.5 py-2.5 rounded-lg border border-red-500/20 text-red-500 text-sm font-medium hover:bg-red-500/8 transition-colors">
                            <LogOut className="w-4 h-4" /> Sign Out
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}

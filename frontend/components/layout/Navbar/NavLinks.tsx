'use client';

import Link from 'next/link';
import { Package, Columns2, ShoppingCart, ShieldCheck, Code2, ChevronDown, Users, BarChart2, PieChart, Layers, GitBranch } from 'lucide-react';
import { useTranslation } from '@/lib/i18n/client';

interface NavLinksProps {
    isActive: (href: string) => boolean;
    exploreOpen: boolean;
    onExploreEnter: () => void;
    onExploreLeave: () => void;
    isExploreActive: boolean;
}

const NAV_LINKS = [
    { href: '/contracts',       label: 'Browse',  icon: Package   },
    { href: '/compare',         label: 'Compare', icon: Columns2  },
    { href: '/marketplace',     label: 'Market',  icon: ShoppingCart },
    { href: '/verify-contract', label: 'Verify',  icon: ShieldCheck },
    { href: '/developer',       label: 'IDE',     icon: Code2 },
] as const;

const EXPLORE_LINKS = [
    { href: '/publishers', label: 'Publishers', icon: Users },
    { href: '/stats', label: 'Statistics', icon: BarChart2 },
    { href: '/analytics', label: 'Analytics', icon: PieChart },
    { href: '/templates', label: 'Templates', icon: Layers },
] as const;

export function NavLinks({ isActive, exploreOpen, onExploreEnter, onExploreLeave, isExploreActive }: NavLinksProps) {
    const { t } = useTranslation('');

    return (
        <div className="hidden md:flex items-center gap-0.5" role="menubar">
            {NAV_LINKS.map(({ href, label, icon: Icon }) => (
                <Link
                    key={href}
                    href={href}
                    role="menuitem"
                    className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[13px] font-medium transition-all ${isActive(href)
                            ? 'text-primary bg-primary/10'
                            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                        }`}
                >
                    <Icon className="w-3 h-3" />
                    {t(`navbar.${label.toLowerCase()}`, label)}
                </Link>
            ))}

            <div className="relative" onMouseEnter={onExploreEnter} onMouseLeave={onExploreLeave}>
                <button
                    className={`flex items-center gap-1 px-3 py-1.5 rounded-md text-[13px] font-medium transition-all focus:outline-none ${isExploreActive || exploreOpen
                            ? 'text-primary bg-primary/10'
                            : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                        }`}
                >
                    Explore
                    <ChevronDown className={`w-3 h-3 transition-transform duration-200 ${exploreOpen ? 'rotate-180' : ''}`} />
                </button>

                <div className={`absolute top-full left-1/2 -translate-x-1/2 w-48 pt-1.5 transition-all duration-150 ${exploreOpen ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-2 pointer-events-none'}`}>
                    <div className="rounded-xl border border-border bg-card shadow-xl overflow-hidden">
                        <div className="py-1.5">
                            {EXPLORE_LINKS.map(({ href, label, icon: Icon }) => (
                                <Link
                                    key={href}
                                    href={href}
                                    className={`flex items-center gap-2.5 px-3 py-2 text-[13px] transition-colors ${isActive(href) ? 'text-primary bg-primary/8' : 'text-muted-foreground hover:text-foreground hover:bg-accent'}`}
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
                className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md text-[13px] font-medium transition-all ${isActive('/graph') ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:text-foreground hover:bg-accent'}`}
            >
                <GitBranch className="w-3 h-3" />
                Graph
            </Link>
        </div>
    );
}

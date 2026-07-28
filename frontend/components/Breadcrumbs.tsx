'use client';

import Link from 'next/link';
import React from 'react';
import { ChevronRight, Home } from 'lucide-react';
import { usePathname } from 'next/navigation';

export interface BreadcrumbItem {
    label: string;
    href?: string;
}

export interface BreadcrumbsProps {
    /** Provide your own crumbs, or omit to auto-generate from the URL path */
    items?: BreadcrumbItem[];
    className?: string;
}

/** Human-readable labels for known path segments */
const SEGMENT_LABELS: Record<string, string> = {
    contracts:            'Contracts',
    compare:              'Compare',
    'verify-contract':    'Verify Contract',
    'verification-status':'Verification Status',
    publishers:           'Publishers',
    stats:                'Statistics',
    analytics:            'Analytics',
    templates:            'Templates',
    graph:                'Dependency Graph',
    publish:              'Publish',
    profile:              'Profile',
    settings:             'Settings',
};

/** Capitalise first letter and replace hyphens */
function prettify(segment: string): string {
    return (
        SEGMENT_LABELS[segment] ??
        segment.replace(/-/g, ' ').replace(/^\w/, c => c.toUpperCase())
    );
}

/** Build breadcrumb items automatically from a pathname string */
function buildCrumbs(pathname: string): BreadcrumbItem[] {
    const parts = pathname.split('/').filter(Boolean);
    return parts.map((segment, idx) => ({
        label: prettify(segment),
        href: '/' + parts.slice(0, idx + 1).join('/'),
    }));
}

export default function Breadcrumbs({ items, className = '' }: BreadcrumbsProps) {
    const pathname = usePathname() ?? '/';
    const crumbs   = items ?? buildCrumbs(pathname);

    // Don't render on the home page
    if (crumbs.length === 0) return null;

    return (
        <nav
            className={`flex items-center text-sm font-medium ${className}`}
            aria-label="Breadcrumb"
        >
            <ol className="flex items-center gap-1 min-w-0 flex-wrap">
                {/* Home */}
                <li className="flex items-center">
                    <Link
                        href="/"
                        className="flex items-center justify-center w-6 h-6 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
                        aria-label="Home"
                    >
                        <Home className="w-3.5 h-3.5" />
                    </Link>
                </li>

                {crumbs.map((item, index) => {
                    const isLast = index === crumbs.length - 1;
                    return (
                        <React.Fragment key={item.label + index}>
                            {/* Separator */}
                            <li className="flex items-center text-muted-foreground/40" aria-hidden="true">
                                <ChevronRight className="w-3.5 h-3.5 flex-shrink-0" />
                            </li>

                            {/* Crumb */}
                            <li className="flex items-center truncate min-w-0">
                                {isLast || !item.href ? (
                                    <span
                                        className="text-foreground font-medium truncate block max-w-[140px] sm:max-w-[280px] md:max-w-none"
                                        aria-current="page"
                                    >
                                        {item.label}
                                    </span>
                                ) : (
                                    <Link
                                        href={item.href}
                                        className="text-muted-foreground hover:text-foreground transition-colors truncate block max-w-[100px] sm:max-w-[200px] md:max-w-none hover:underline underline-offset-2"
                                    >
                                        {item.label}
                                    </Link>
                                )}
                            </li>
                        </React.Fragment>
                    );
                })}
            </ol>
        </nav>
    );
}

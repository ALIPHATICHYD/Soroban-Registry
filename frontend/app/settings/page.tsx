'use client';

import Navbar from '@/components/Navbar';
import { useTheme, Theme } from '@/hooks/useTheme';
import { Sun, Moon, Monitor, Shield, Bell, User, Globe, Lock, Palette } from 'lucide-react';
import { useTranslation } from '@/lib/i18n/client';

export default function SettingsPage() {
    const { theme, setTheme } = useTheme();
    const { t } = useTranslation('common');

    const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
        { value: 'light', label: 'Light', icon: Sun },
        { value: 'dark', label: 'Dark', icon: Moon },
        { value: 'system', label: 'System', icon: Monitor },
    ];

    return (
        <div className="min-h-screen bg-background text-foreground">
            <Navbar />

            <main className="max-w-4xl mx-auto px-4 py-12 sm:px-6 lg:px-8">
                <div className="mb-10">
                    <h1 className="text-3xl font-bold tracking-tight mb-2">Settings</h1>
                    <p className="text-muted-foreground text-lg">Manage your registry experience and preferences.</p>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-[240px_1fr] gap-10">
                    {/* Sidebar Nav */}
                    <aside className="space-y-1">
                        {[
                            { label: 'General', icon: Palette, active: true },
                            { label: 'Account', icon: User },
                            { label: 'Notifications', icon: Bell },
                            { label: 'Privacy & Security', icon: Shield },
                            { label: 'Language', icon: Globe },
                            { label: 'API Keys', icon: Lock },
                        ].map((item) => (
                            <button
                                key={item.label}
                                className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                                    item.active 
                                        ? 'bg-primary/10 text-primary' 
                                        : 'text-muted-foreground hover:text-foreground hover:bg-accent'
                                }`}
                            >
                                <item.icon className="w-4 h-4" />
                                {item.label}
                            </button>
                        ))}
                    </aside>

                    {/* Content */}
                    <div className="space-y-8">
                        {/* Appearance Section */}
                        <section className="bg-card border border-border rounded-2xl overflow-hidden shadow-sm">
                            <div className="px-6 py-5 border-b border-border">
                                <h2 className="text-lg font-semibold flex items-center gap-2">
                                    <Palette className="w-5 h-5 text-primary" />
                                    Appearance
                                </h2>
                                <p className="text-sm text-muted-foreground mt-1">Customize how the registry looks for you.</p>
                            </div>

                            <div className="p-6 space-y-6">
                                <div>
                                    <label className="text-sm font-medium mb-4 block">Color Theme</label>
                                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                                        {themeOptions.map((option) => (
                                            <button
                                                key={option.value}
                                                onClick={() => setTheme(option.value)}
                                                className={`flex flex-col items-center gap-3 p-4 rounded-xl border-2 transition-all group ${
                                                    theme === option.value
                                                        ? 'border-primary bg-primary/5'
                                                        : 'border-border hover:border-primary/50 hover:bg-accent'
                                                }`}
                                            >
                                                <div className={`p-3 rounded-full transition-colors ${
                                                    theme === option.value ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground group-hover:text-foreground'
                                                }`}>
                                                    <option.icon className="w-6 h-6" />
                                                </div>
                                                <span className="font-medium">{option.label}</span>
                                                {theme === option.value && (
                                                    <div className="absolute top-2 right-2 w-2 h-2 rounded-full bg-primary" />
                                                )}
                                            </button>
                                        ))}
                                    </div>
                                </div>

                                <div className="pt-4 border-t border-border">
                                    <div className="flex items-center justify-between">
                                        <div>
                                            <p className="font-medium">Reduced Motion</p>
                                            <p className="text-sm text-muted-foreground">Minimize animations across the interface.</p>
                                        </div>
                                        <button className="w-12 h-6 bg-muted rounded-full relative p-1 transition-colors hover:bg-muted/80">
                                            <div className="w-4 h-4 bg-background rounded-full shadow-sm" />
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </section>

                        {/* Language Section (Mock) */}
                        <section className="bg-card border border-border rounded-2xl overflow-hidden shadow-sm">
                            <div className="px-6 py-5 border-b border-border">
                                <h2 className="text-lg font-semibold flex items-center gap-2">
                                    <Globe className="w-5 h-5 text-secondary" />
                                    Language & Regional
                                </h2>
                            </div>
                            <div className="p-6">
                                <p className="text-sm text-muted-foreground mb-4">Select your preferred language for the interface.</p>
                                <select className="w-full sm:w-64 bg-background border border-border rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-primary outline-none">
                                    <option>English (US)</option>
                                    <option>Arabic</option>
                                    <option>Spanish</option>
                                    <option>French</option>
                                </select>
                            </div>
                        </section>

                        <div className="flex justify-end pt-4">
                            <button className="px-6 py-2 rounded-xl bg-primary text-primary-foreground font-semibold btn-glow hover:brightness-110 transition-all">
                                Save Changes
                            </button>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    );
}

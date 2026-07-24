'use client';

import Image from 'next/image';
import { useEffect, useState } from 'react';
import { ContractExample, api } from '@/lib/api';
import { generateBlurHashPlaceholder, generateSolidPlaceholder } from '@/lib/images';
import CodeRunner from './CodeRunner';
import { ThumbsUp, ThumbsDown } from 'lucide-react';

interface ExampleCardProps {
  example: ContractExample;
}

type ExampleWithOptionalImage = ContractExample & {
  repo_avatar_url?: string;
  repo_avatar_blurhash?: string;
  repo_avatar_placeholder_color?: string;
  thumbnail_url?: string;
  thumbnail_blurhash?: string;
  thumbnail_placeholder_color?: string;
};

export default function ExampleCard({ example }: ExampleCardProps) {
  const [activeTab, setActiveTab] = useState<'js' | 'rust'>('js');
  const [rating, setRating] = useState<number | null>(null); // Just for UI feedback
  const [isRating, setIsRating] = useState(false);
  const [avatarPlaceholder, setAvatarPlaceholder] = useState<string | null>(null);
  const [avatarLoadError, setAvatarLoadError] = useState(false);

  // If no JS code, default to Rust
  const effectiveTab = example.code_js ? activeTab : 'rust';
  const hasMultipleLangs = !!(example.code_js && example.code_rust);
  const exampleWithImage = example as ExampleWithOptionalImage;
  const avatarSrc = exampleWithImage.repo_avatar_url ?? exampleWithImage.thumbnail_url;
  const avatarBlurHash =
    exampleWithImage.repo_avatar_blurhash ?? exampleWithImage.thumbnail_blurhash;
  const avatarFallbackColor =
    exampleWithImage.repo_avatar_placeholder_color ??
    exampleWithImage.thumbnail_placeholder_color ??
    '#e5e7eb';

  useEffect(() => {
    requestAnimationFrame(() => {
      setAvatarLoadError(false);

      if (!avatarSrc) {
        setAvatarPlaceholder(null);
        return;
      }

      if (avatarBlurHash) {
        setAvatarPlaceholder(
          generateBlurHashPlaceholder(avatarBlurHash, {
            width: 24,
            height: 24,
            fallbackColor: avatarFallbackColor,
          })
        );
        return;
      }

      setAvatarPlaceholder(generateSolidPlaceholder(avatarFallbackColor));
    });
  }, [avatarSrc, avatarBlurHash, avatarFallbackColor]);

  const handleRate = async (val: number) => {
    try {
      setIsRating(true);
      // TODO: Replace with real auth user ID once authentication is implemented
      const userId = localStorage.getItem('user_id') || crypto.randomUUID();
      localStorage.setItem('user_id', userId);
      
      await api.rateExample(example.id, userId, val);
      setRating(val);
    } catch {
      // Rating failed — silently ignore to avoid disrupting UX
    } finally {
      setIsRating(false);
    }
  };

  return (
    <div className="bg-card rounded-2xl border border-border overflow-hidden">
      <div className="p-6 border-b border-border">
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-start gap-3 min-w-0">
            {avatarSrc && !avatarLoadError ? (
              <div
                className="relative h-12 w-12 shrink-0 overflow-hidden rounded-full border border-border bg-accent"
                style={{ backgroundColor: avatarFallbackColor }}
              >
                <Image
                  src={avatarSrc}
                  alt={`${example.title} repository avatar`}
                  fill
                  className="object-cover"
                  sizes="48px"
                  onError={() => setAvatarLoadError(true)}
                  placeholder={avatarPlaceholder ? 'blur' : 'empty'}
                  blurDataURL={avatarPlaceholder ?? undefined}
                />
              </div>
            ) : null}

            <div className="min-w-0">
              <h3 className="text-xl font-bold text-foreground mb-2 truncate">
                {example.title}
              </h3>
              <span className={`inline-block px-2 py-1 rounded text-xs font-medium uppercase tracking-wide ${
                example.category === 'basic' ? 'bg-green-500/10 text-green-600 dark:text-green-400' :
                example.category === 'advanced' ? 'bg-secondary/10 text-secondary' :
                'bg-primary/10 text-primary'
              }`}>
                {example.category}
              </span>
            </div>
          </div>
          
          <div className="flex items-center gap-2">
            <button
              onClick={() => handleRate(1)}
              disabled={isRating || rating === 1}
              className={`flex items-center gap-1 p-2 rounded-lg transition-colors ${
                rating === 1 ? 'bg-green-500/10 text-green-600' : 'hover:bg-accent text-muted-foreground'
              }`}
            >
              <ThumbsUp className="w-5 h-5" />
              <span className="text-sm font-medium">{example.rating_up + (rating === 1 ? 1 : 0)}</span>
            </button>
            <button
              onClick={() => handleRate(-1)}
              disabled={isRating || rating === -1}
              className={`flex items-center gap-1 p-2 rounded-lg transition-colors ${
                rating === -1 ? 'bg-red-500/10 text-red-600' : 'hover:bg-accent text-muted-foreground'
              }`}
            >
              <ThumbsDown className="w-5 h-5" />
              <span className="text-sm font-medium">{example.rating_down + (rating === -1 ? 1 : 0)}</span>
            </button>
          </div>
        </div>

        {example.description && (
          <p className="text-muted-foreground">
            {example.description}
          </p>
        )}
      </div>

      <div className="p-6">
        {hasMultipleLangs && (
          <div className="flex items-center gap-4 mb-4 border-b border-border">
            <button
              onClick={() => setActiveTab('js')}
              className={`pb-2 text-sm font-medium transition-colors border-b-2 ${
                effectiveTab === 'js'
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              JavaScript / TypeScript
            </button>
            <button
              onClick={() => setActiveTab('rust')}
              className={`pb-2 text-sm font-medium transition-colors border-b-2 ${
                effectiveTab === 'rust'
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              Rust
            </button>
          </div>
        )}

        {effectiveTab === 'js' && example.code_js && (
          <CodeRunner
            initialCode={example.code_js}
            language="javascript"
            // Sent with copy events so analytics can identify the source example.
            copyAnalytics={{
              contractId: example.contract_id,
              exampleId: example.id,
              exampleTitle: example.title,
            }}
          />
        )}

        {effectiveTab === 'rust' && example.code_rust && (
          <CodeRunner
            initialCode={example.code_rust}
            language="rust"
            // Same metadata for Rust tab copies.
            copyAnalytics={{
              contractId: example.contract_id,
              exampleId: example.id,
              exampleTitle: example.title,
            }}
          />
        )}
      </div>
    </div>
  );
}

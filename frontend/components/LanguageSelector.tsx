'use client'

import { useTranslation } from '@/lib/i18n/client'
import { languages } from '@/lib/i18n/settings'
import { useState, useEffect } from 'react'

export default function LanguageSelector({ lng }: { lng: string }) {
  const { t, i18n } = useTranslation(lng)
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    requestAnimationFrame(() => setMounted(true))
  }, [])

  if (!mounted) return null

  return (
    <div className="relative inline-block text-left group">
      <button
        type="button"
        className="inline-flex justify-center w-full px-3 py-1.5 text-[13px] font-medium text-foreground bg-background border border-border rounded-md shadow-sm hover:bg-accent focus:outline-none transition-all"
        id="language-menu-button"
        aria-expanded="true"
        aria-haspopup="true"
      >
        {lng.toUpperCase()}
        <svg
          className="w-4 h-4 ml-2 -mr-1"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            fillRule="evenodd"
            d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
            clipRule="evenodd"
          />
        </svg>
      </button>

      <div
        className="absolute right-0 w-32 mt-2 origin-top-right bg-card border border-border rounded-xl shadow-xl focus:outline-none hidden group-hover:block z-50 overflow-hidden"
        role="menu"
        aria-orientation="vertical"
        aria-labelledby="language-menu-button"
        tabIndex={-1}
      >
        <div className="py-1" role="none">
          {languages.map((l) => (
            <button
              key={l}
              onClick={() => i18n.changeLanguage(l)}
              className={`${
                lng === l ? 'bg-primary/10 text-primary font-semibold' : 'text-muted-foreground'
              } block px-4 py-2 text-sm w-full text-left hover:bg-accent transition-colors`}
              role="menuitem"
              tabIndex={-1}
            >
              {l === 'en' && 'English'}
              {l === 'es' && 'Español'}
              {l === 'fr' && 'Français'}
              {l === 'ar' && 'العربية'}
            </button>
          ))}
        </div>
      </div>
    </div>
  )
}

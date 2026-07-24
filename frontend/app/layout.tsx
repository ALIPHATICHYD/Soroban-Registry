import type { Metadata } from "next";
import "./globals.css";
import Providers from "@/components/Providers";
import Script from "next/script";
import PageViewTracker from "@/components/PageViewTracker";
import UserInteractionTracker from "@/components/UserInteractionTracker";
import { cookies, headers } from "next/headers";
import { fallbackLng, languages, cookieName } from "@/lib/i18n/settings";
import acceptLanguage from 'accept-language';

acceptLanguage.languages(languages);

const GA_PROVIDER = process.env.NEXT_PUBLIC_ANALYTICS_PROVIDER || 'ga'
const GA_ID = process.env.NEXT_PUBLIC_GA_ID

export const metadata: Metadata = {
  metadataBase: new URL("https://soroban-registry.com"),
  title: {
    default: "Soroban Registry – Discover & Publish Stellar Smart Contracts",
    template: "%s – Soroban Registry",
  },
  description:
    "Explore, publish, and verify Soroban smart contracts on Stellar. The trusted registry for reusable smart contract packages.",
  applicationName: "Soroban Registry",
  keywords: [
    "soroban",
    "stellar",
    "smart contracts",
    "stellar blockchain",
    "contract registry",
    "stellar developer tools",
    "web3 packages",
  ],
  authors: [{ name: "Soroban Registry" }],
  robots: {
    index: true,
    follow: true,
  },
  alternates: {
    canonical: "https://soroban-registry.com",
  },
  openGraph: {
    title: "Soroban Registry – Discover & Publish Stellar Smart Contracts",
    description:
      "Explore, publish, and verify Soroban smart contracts on Stellar. The trusted registry for reusable smart contract packages.",
    url: "https://soroban-registry.com",
    siteName: "Soroban Registry",
    images: [
      {
        url: "https://soroban-registry.com/og-image.png",
        width: 1200,
        height: 630,
        alt: "Soroban Registry – Discover and Publish Soroban Smart Contracts",
      },
    ],
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "Soroban Registry – Discover & Publish Stellar Smart Contracts",
    description:
      "Explore, publish, and verify Soroban smart contracts on Stellar. The trusted registry for reusable smart contract packages.",
    images: ["https://soroban-registry.com/og-image.png"],
  },
};

export default async function RootLayout({ children }: { children: React.ReactNode }) {
  const cookieStore = await cookies();
  let lng = cookieStore.get(cookieName)?.value;
  
  if (!lng) {
    const headersList = await headers();
    lng = acceptLanguage.get(headersList.get('accept-language')) || fallbackLng;
  }

  const dir = lng === 'ar' ? 'rtl' : 'ltr';

  return (
    <html lang={lng} dir={dir} suppressHydrationWarning>
      <head>
        {/* Theme detection script to prevent flash */}
        <script
          dangerouslySetInnerHTML={{
            __html: `
              (function() {
                try {
                  var theme = localStorage.getItem('soroban-registry-theme');
                  var supportDarkMode = window.matchMedia('(prefers-color-scheme: dark)').matches;
                  if (theme === 'dark' || (theme === 'system' && supportDarkMode) || (!theme && supportDarkMode)) {
                    document.documentElement.classList.add('dark');
                  } else {
                    document.documentElement.classList.remove('dark');
                  }
                } catch (e) {}
              })();
            `,
          }}
        />
        {/* Only load GA script if GA is selected */}
        {GA_PROVIDER === 'ga' && GA_ID && (
          <>
            <Script
              strategy="afterInteractive"
              src={`https://www.googletagmanager.com/gtag/js?id=${GA_ID}`}
            />
            <Script
              id="ga-init"
              strategy="afterInteractive"
              dangerouslySetInnerHTML={{
                __html: `
                  window.dataLayer = window.dataLayer || [];
                  function gtag(){dataLayer.push(arguments);}
                  gtag('js', new Date());
                  gtag('config', '${GA_ID}', { send_page_view: false });
                `,
              }}
            />
          </>
        )}
      </head>
      <body className="font-sans antialiased">
        <Providers>
          {children}

          {/* called on every page to track page views */}
          <PageViewTracker />
          {/* tracks external link clicks, form submissions, and client runtime errors */}
          <UserInteractionTracker />
        </Providers>
      </body>
    </html>
  )
}

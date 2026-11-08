import type { Metadata, Viewport } from "next";
import { Inter, Roboto_Mono } from "next/font/google";
import localFont from "next/font/local";
import Script from "next/script";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], display: "swap", variable: "--font-inter" });
const robotoMono = Roboto_Mono({ subsets: ["latin"], weight: ["400", "700"], variable: "--font-mono" });
const calSans = localFont({
  src: [{ path: "../public/fonts/CalSans-SemiBold.woff2", weight: "600", style: "normal" }],
  variable: "--font-display",
  display: "swap",
  adjustFontFallback: "Arial",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://fixture.example.com"),
  title: { default: "Yolo Fixture", template: "%s | Yolo Fixture" },
  description: "Next.js 16 migration fixture",
  applicationName: "Yolo Fixture",
  keywords: ["yolo", "next", "bun"],
  authors: [{ name: "Aphrody", url: "https://aphrody.com" }],
  openGraph: {
    title: "Yolo Fixture",
    description: "Migration fixture",
    url: "https://fixture.example.com",
    siteName: "Yolo Fixture",
    images: [{ url: "/opengraph-image", width: 1200, height: 630 }],
    locale: "en_US",
    type: "website",
  },
  twitter: { card: "summary_large_image", site: "@aphrody" },
  robots: { index: true, follow: true },
  alternates: { canonical: "/", languages: { "fr-FR": "/fr" } },
  icons: { icon: "/icon.svg", apple: "/apple-icon.png" },
  manifest: "/manifest.webmanifest",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "#09090b" },
    { media: "(prefers-color-scheme: light)", color: "#ffffff" },
  ],
  colorScheme: "dark light",
};

export default function RootLayout({
  children,
  modal,
}: {
  children: React.ReactNode;
  modal: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${inter.variable} ${robotoMono.variable} ${calSans.variable}`}>
      <body className={inter.className}>
        {children}
        {modal}
        <Script src="https://analytics.example.com/script.js" strategy="afterInteractive" />
      </body>
    </html>
  );
}

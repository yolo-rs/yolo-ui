import type { ReactNode } from "react";
import localFont from "next/font/local";

const sans = localFont({ src: "./fonts/Fixture-Sans.woff2", variable: "--font-sans", display: "swap" });

export const metadata = { title: "Fixture" };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={sans.variable}>
      <body className={sans.className}>{children}</body>
    </html>
  );
}

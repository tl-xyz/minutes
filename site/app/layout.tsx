import type { Metadata } from "next";
import { GeistSans } from "geist/font/sans";
import { GeistMono } from "geist/font/mono";
import "./globals.css";

export const metadata: Metadata = {
  title: "minutes — your AI remembers every conversation",
  description:
    "Open-source, privacy-first conversation memory for AI assistants. Record meetings, capture voice memos, search everything. Local transcription, structured markdown, Claude-native.",
  metadataBase: new URL("https://useminutes.app"),
  alternates: { canonical: "/" },
  openGraph: {
    title: "minutes — your AI remembers every conversation",
    description:
      "Open-source conversation memory. Local transcription, structured markdown, AI-native.",
    type: "website",
    url: "https://useminutes.app",
  },
  other: {
    "theme-color": "#000000",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${GeistSans.variable} ${GeistMono.variable}`}>
      <head>
        <link rel="alternate" type="text/plain" href="/llms.txt" />
      </head>
      <body className="font-sans antialiased">{children}</body>
    </html>
  );
}

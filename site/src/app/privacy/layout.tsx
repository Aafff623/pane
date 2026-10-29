export const metadata = {
  title: { absolute: "Privacy — Extension storage, Cloud relay and analytics" },
  description:
    "How Design Mode stores browser edits, when Cloud or Local MCP traffic starts, what the relay retains, and which analytics events the website sends.",
  alternates: { canonical: "https://designmode.app/privacy" },
  openGraph: {
    type: "website",
    title: "Privacy — Extension storage, Cloud relay and analytics",
    description:
      "How Design Mode stores browser edits, when Cloud or Local MCP traffic starts, what the relay retains, and which analytics events the website sends.",
    url: "https://designmode.app/privacy",
    images: [
      {
        url: "/og-image.png",
        width: 1200,
        height: 630,
        alt: "Design Mode — The visual editor for all your agent’s work",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Privacy — Extension storage, Cloud relay and analytics",
    description:
      "How Design Mode stores browser edits, when Cloud or Local MCP traffic starts, what the relay retains, and which analytics events the website sends.",
    images: [
      {
        url: "/og-image.png",
        width: 1200,
        height: 630,
        alt: "Design Mode — The visual editor for all your agent’s work",
      },
    ],
  },
};

export default function PrivacyLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}

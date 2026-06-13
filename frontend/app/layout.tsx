import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Kedge | Autonomous Claim Adjuster",
  description:
    "A zero-knowledge clearinghouse for autonomous parametric shipping insurance.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}

import type { Metadata } from "next";
import localFont from "next/font/local";
import Link from "next/link";
import "./globals.css";

const geistSans = localFont({
  src: "./fonts/GeistVF.woff",
  variable: "--font-geist-sans",
  weight: "100 900",
});
const geistMono = localFont({
  src: "./fonts/GeistMonoVF.woff",
  variable: "--font-geist-mono",
  weight: "100 900",
});

export const metadata: Metadata = {
  title: "NIER Factory Dashboard",
  description: "Real-time factory floor monitoring and analytics",
};

function NavLink({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <Link
      href={href}
      className="px-4 py-2 text-sm font-medium text-gray-300 hover:text-white hover:bg-white/5 rounded-lg transition-colors"
    >
      {children}
    </Link>
  );
}

function StatusIndicator({ status, label }: { status: "online" | "warning" | "offline"; label: string }) {
  const colors = {
    online: "bg-green-500",
    warning: "bg-yellow-500",
    offline: "bg-red-500",
  };

  return (
    <div className="flex items-center gap-2 text-sm text-gray-400">
      <span className={`w-2 h-2 rounded-full ${colors[status]} animate-pulse-live`} />
      {label}
    </div>
  );
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark">
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased min-h-screen bg-[#0a0a0f]`}
      >
        <div className="flex flex-col min-h-screen">
          {/* Header */}
          <header className="sticky top-0 z-50 border-b border-[#1f1f2e] bg-[#0a0a0f]/95 backdrop-blur supports-[backdrop-filter]:bg-[#0a0a0f]/60">
            <div className="flex h-16 items-center justify-between px-6">
              <div className="flex items-center gap-8">
                <Link href="/" className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center">
                    <svg className="w-5 h-5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                  </div>
                  <span className="text-lg font-semibold text-white">NIER</span>
                </Link>
                <nav className="flex items-center gap-1">
                  <NavLink href="/">Dashboard</NavLink>
                  <NavLink href="/live">Live View</NavLink>
                  <NavLink href="/analytics">Analytics</NavLink>
                  <NavLink href="/alerts">Alerts</NavLink>
                </nav>
              </div>
              <div className="flex items-center gap-6">
                <StatusIndicator status="online" label="12 Cameras Online" />
                <StatusIndicator status="online" label="API Connected" />
                <div className="text-sm text-gray-500 font-mono">
                  {new Date().toLocaleTimeString()}
                </div>
              </div>
            </div>
          </header>

          {/* Main Content */}
          <main className="flex-1 grid-pattern">
            {children}
          </main>

          {/* Footer */}
          <footer className="border-t border-[#1f1f2e] bg-[#0a0a0f] py-4 px-6">
            <div className="flex items-center justify-between text-sm text-gray-500">
              <span>NIER Factory Analytics Platform</span>
              <span>v1.0.0</span>
            </div>
          </footer>
        </div>
      </body>
    </html>
  );
}

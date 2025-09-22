import '../styles/globals.css'

export const metadata = {
  title: 'Crypto Trading Dashboard',
  description: 'Real-time cryptocurrency trading dashboard with multi-exchange support',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" className='dark'>
      <body className="font-sans">
        <div className="min-h-screen bg-background">
          <header className="border-b border-border bg-card">
            <div className="container mx-auto px-4 py-4">
              <h1 className="text-2xl font-bold text-foreground">
                Crypto Trading Dashboard
              </h1>
            </div>
          </header>
          <main className="container mx-auto px-4 py-8">
            {children}
          </main>
        </div>
      </body>
    </html>
  )
}
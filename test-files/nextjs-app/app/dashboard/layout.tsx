export default function DashboardLayout({
  children,
  analytics,
}: {
  children: React.ReactNode
  analytics: React.ReactNode
}) {
  return (
    <div className="flex">
      <aside className="w-64 bg-gray-100 p-4">
        <h2>Dashboard Sidebar</h2>
      </aside>
      <main className="flex-1 p-4">
        {children}
        <div className="mt-8">{analytics}</div>
      </main>
    </div>
  )
}

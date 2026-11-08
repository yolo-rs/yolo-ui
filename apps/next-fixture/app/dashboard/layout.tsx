export default function DashboardLayout({
  children,
  analytics,
}: {
  children: React.ReactNode;
  analytics: React.ReactNode;
}) {
  return (
    <div>
      <nav>Dashboard</nav>
      {children}
      <aside>{analytics}</aside>
    </div>
  );
}

export default function MarketingLayout({ children }: { children: React.ReactNode }) {
  return (
    <div data-marketing>
      <header>Marketing</header>
      {children}
      <footer>Yolo Fixture</footer>
    </div>
  );
}

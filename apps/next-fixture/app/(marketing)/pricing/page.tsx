export const metadata = { title: "Pricing" };
// Deprecated in Next.js 16: the edge runtime segment config must be reported by the migrator.
export const runtime = "edge";

export default function PricingPage() {
  return (
    <main>
      <h1>Pricing</h1>
      <ul>
        <li>Free: 0 USD</li>
        <li>Pro: 12 USD per month</li>
      </ul>
    </main>
  );
}

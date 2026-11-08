export const fetchCache = "force-cache";

export default async function ShopPage({
  params,
  searchParams,
}: {
  params: Promise<{ filters?: string[] }>;
  searchParams: Promise<{ sort?: string }>;
}) {
  const { filters = [] } = await params;
  const { sort } = await searchParams;
  return (
    <main>
      <h1>Shop</h1>
      <p>{filters.length ? filters.join(", ") : "all products"}</p>
      <p>sorted by {sort ?? "relevance"}</p>
    </main>
  );
}

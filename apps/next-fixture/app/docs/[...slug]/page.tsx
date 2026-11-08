export const dynamic = "force-static";

export default async function DocsPage({ params }: { params: Promise<{ slug: string[] }> }) {
  const { slug } = await params;
  return (
    <article>
      <h1>Docs</h1>
      <p>{slug.join(" / ")}</p>
    </article>
  );
}

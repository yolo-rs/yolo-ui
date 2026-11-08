import Image from "next/image";
import Link from "next/link";
import { Suspense } from "react";
import { getPosts } from "./lib/data";
import { Counter } from "./components/Counter";
import hero from "@/public/hero.png";

export const revalidate = 3600;

export default async function HomePage({
  searchParams,
}: {
  searchParams: Promise<{ q?: string }>;
}) {
  const { q } = await searchParams;
  const posts = await getPosts();
  return (
    <main>
      <Image src={hero} alt="Hero" priority sizes="(max-width: 768px) 100vw, 50vw" placeholder="blur" />
      <Image src="https://images.unsplash.com/photo-1" alt="Remote" width={800} height={600} quality={90} />
      <Image src="https://untrusted.example.net/x.png" alt="Unmatched" width={100} height={100} />
      <ul>
        {posts.map((p) => (
          <li key={p.slug}>
            <Link href={`/blog/${p.slug}`} prefetch={false}>
              {p.title}
            </Link>
          </li>
        ))}
      </ul>
      <Suspense fallback={<p>Loading counter</p>}>
        <Counter initial={posts.length} />
      </Suspense>
      <p>{q ?? "no query"}</p>
    </main>
  );
}

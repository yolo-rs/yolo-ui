import type { Metadata, ResolvingMetadata } from "next";
import { notFound } from "next/navigation";
import { getPost, getPosts } from "../../lib/data";

export const dynamicParams = true;
export const revalidate = 60;

export async function generateStaticParams() {
  const posts = await getPosts();
  return posts.map((p) => ({ slug: p.slug }));
}

export async function generateMetadata(
  { params }: { params: Promise<{ slug: string }> },
  parent: ResolvingMetadata,
): Promise<Metadata> {
  const { slug } = await params;
  const post = await getPost(slug);
  const previous = (await parent).openGraph?.images ?? [];
  return { title: post?.title ?? "Post", openGraph: { images: ["/blog-og.png", ...previous] } };
}

export default async function BlogPost({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const post = await getPost(slug);
  if (!post) notFound();
  return (
    <article>
      <h1>{post.title}</h1>
      <p>{post.body}</p>
    </article>
  );
}

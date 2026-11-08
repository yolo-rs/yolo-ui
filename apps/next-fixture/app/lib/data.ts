import { cacheLife, cacheTag, unstable_cache } from "next/cache";

export interface Post {
  slug: string;
  title: string;
  body: string;
}

export async function getPosts(): Promise<Post[]> {
  "use cache";
  cacheLife("blog");
  cacheTag("posts");
  return [
    { slug: "hello-yolo", title: "Hello Yolo", body: "First post" },
    { slug: "bun-vs-node", title: "Bun vs Node", body: "Second post" },
  ];
}

export async function getPost(slug: string) {
  "use cache";
  cacheLife("hours");
  cacheTag("posts", `post:${slug}`);
  const posts = await getPosts();
  return posts.find((p) => p.slug === slug) ?? null;
}

export const getStats = unstable_cache(async () => ({ visits: 42 }), ["stats"], {
  revalidate: 60,
  tags: ["stats"],
});

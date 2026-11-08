import type { MetadataRoute } from "next";

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    {
      url: "https://fixture.example.com",
      lastModified: new Date("2026-01-01"),
      changeFrequency: "weekly",
      priority: 1,
    },
    {
      url: "https://fixture.example.com/blog/hello-yolo",
      lastModified: new Date("2026-01-02"),
      changeFrequency: "monthly",
      priority: 0.8,
    },
  ];
}

// Shared page content for the Bun lane. The Leptos lane parses the same JSON
// (lane-leptos/src/content.rs) and expands the legal sections with the same rule.
import site from "../content/site.json";

export { site };

export interface LegalSection {
  id: string;
  title: string;
  paragraphs: string[];
}

export function legalSections(): LegalSection[] {
  const { sections, topics, paragraphs } = site.legal;
  return Array.from({ length: sections }, (_, i) => ({
    id: `art-${i + 1}`,
    title: `${i + 1}. ${topics[i % topics.length]}${i >= topics.length ? " (suite)" : ""}`,
    paragraphs: [0, 1, 2].map(k => paragraphs[(i * 3 + k) % paragraphs.length]!),
  }));
}

export function href(kit: string, path: string): string {
  return path ? `/${kit}/${path}` : `/${kit}/`;
}

// Page shell built from @aphrody/shenron-ui server-safe layout components (read-only kit source).
import type { ReactNode } from "react";
import { PageShell } from "@kit/layout/PageShell";
import { FooterFrame } from "@kit/navigation/FooterFrame";
import { TabNav } from "@kit/navigation/TabNav";
import { DragonBall } from "@kit/toriyama/DragonBall";
import { href, site } from "../../shared/content";

const KIT = "shenron";

export function Shell({ active, children }: { active: string; children: ReactNode }) {
  const items = site.nav.map(n => ({ href: href(KIT, n.path), label: n.label }));
  const current = href(KIT, site.nav.find(n => n.id === active)?.path ?? "");
  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <header className="border-b border-[var(--border)] bg-[var(--card)]">
        <PageShell rhythm="none" className="flex flex-wrap items-center justify-between gap-4 py-3">
          <a href={href(KIT, "")} className="flex items-center gap-2 font-title text-xl font-extrabold text-dbz-orange">
            <DragonBall stars={4} size={28} decorative />
            {site.brand}
          </a>
          <TabNav items={items} active={current} ariaLabel="Navigation principale" />
        </PageShell>
      </header>
      <PageShell as="main" rhythm="tight" className="w-full flex-1">
        {children}
      </PageShell>
      <FooterFrame>
        <PageShell rhythm="none" className="py-6 text-sm text-muted-foreground">
          {site.footer}
        </PageShell>
      </FooterFrame>
    </div>
  );
}

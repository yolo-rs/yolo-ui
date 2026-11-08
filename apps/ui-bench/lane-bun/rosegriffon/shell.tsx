// Page shell built from @rosegriffon/ui components (imported read-only from the kit source).
import type { ReactNode } from "react";
import { Button } from "@kit/components/button";
import { Separator } from "@kit/components/separator";
import { href, site } from "../../shared/content";

const KIT = "rosegriffon";

export function Shell({ active, children }: { active: string; children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <header className="border-b border-outline-variant bg-surface-container-low">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-3">
          <a href={href(KIT, "")} className="type-title-large text-primary">
            {site.brand}
          </a>
          <nav aria-label="Navigation principale" className="flex flex-wrap items-center gap-1">
            {site.nav.map(item => (
              <Button
                key={item.id}
                asChild
                variant={item.id === active ? "secondary" : "ghost"}
                size="sm"
              >
                <a href={href(KIT, item.path)} aria-current={item.id === active ? "page" : undefined}>
                  {item.label}
                </a>
              </Button>
            ))}
          </nav>
        </div>
      </header>
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8">{children}</main>
      <footer className="bg-surface-container">
        <Separator />
        <p className="mx-auto max-w-6xl px-4 py-6 type-body-small text-on-surface-variant">{site.footer}</p>
      </footer>
    </div>
  );
}

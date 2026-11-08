import { createRoot } from "react-dom/client";
import { Accordion, AccordionItem, AccordionPanel, AccordionTrigger } from "@kit/components/accordion";
import { Badge } from "@kit/components/badge";
import { Button } from "@kit/components/button";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "@kit/components/card";
import { SectionHeading } from "@kit/layout/SectionHeading";
import { StatTiles } from "@kit/public/StatTiles";
import { href, site } from "../../shared/content";
import { Shell } from "./shell";

function Home() {
  const { hero } = site;
  return (
    <Shell active="home">
      <section className="rounded-2xl border-2 border-black bg-[var(--card)] px-6 py-10 shadow-[6px_6px_0px_#000]">
        <p className="text-xs font-extrabold uppercase tracking-wider text-dbz-orange">{hero.eyebrow}</p>
        <h1 className="mt-2 font-title text-4xl font-extrabold">{hero.title}</h1>
        <p className="mt-4 max-w-3xl text-lg text-muted-foreground">{hero.lead}</p>
        <div className="mt-6 flex flex-wrap gap-3">
          <Button size="lg" nativeButton={false} render={<a href={href("shenron", "gallery")} />}>
            {hero.primary}
          </Button>
          <Button size="lg" variant="outline" nativeButton={false} render={<a href={href("shenron", "legal")} />}>
            {hero.secondary}
          </Button>
        </div>
      </section>

      <StatTiles className="mt-8" items={site.stats.map(s => ({ label: s.label, value: s.value }))} />

      <section aria-label="Points clés" className="mt-8 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {site.cards.map(c => (
          <Card key={c.title}>
            <CardHeader>
              <Badge variant="secondary">{c.tag}</Badge>
              <CardTitle>{c.title}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">{c.body}</p>
            </CardContent>
            <CardFooter>
              <Button variant="outline" size="sm">
                {c.cta}
              </Button>
            </CardFooter>
          </Card>
        ))}
      </section>

      <section className="mt-10">
        <SectionHeading title="Questions fréquentes" />
        <Accordion className="mt-4 grid gap-3">
          {site.faq.map((f, i) => (
            <AccordionItem key={f.id} value={f.id}>
              <AccordionTrigger data-bench-click={i === 0 ? "" : undefined}>{f.q}</AccordionTrigger>
              <AccordionPanel>{f.a}</AccordionPanel>
            </AccordionItem>
          ))}
        </Accordion>
      </section>
    </Shell>
  );
}

createRoot(document.getElementById("root")!).render(<Home />);

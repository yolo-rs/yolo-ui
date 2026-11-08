import { createRoot } from "react-dom/client";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@kit/components/accordion";
import { Badge } from "@kit/components/badge";
import { Button } from "@kit/components/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@kit/components/card";
import { href, site } from "../../shared/content";
import { Shell } from "./shell";

function Home() {
  const { hero } = site;
  return (
    <Shell active="home">
      <section className="rounded-(--md-sys-shape-corner-extra-large) bg-primary-container px-6 py-10 text-on-primary-container">
        <p className="type-label-large uppercase">{hero.eyebrow}</p>
        <h1 className="mt-2 type-display-small">{hero.title}</h1>
        <p className="mt-4 max-w-3xl type-body-large">{hero.lead}</p>
        <div className="mt-6 flex flex-wrap gap-3">
          <Button asChild size="lg">
            <a href={href("rosegriffon", "gallery")}>{hero.primary}</a>
          </Button>
          <Button asChild size="lg" variant="outline">
            <a href={href("rosegriffon", "legal")}>{hero.secondary}</a>
          </Button>
        </div>
      </section>

      <section aria-label="Chiffres" className="mt-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        {site.stats.map(s => (
          <Card key={s.label}>
            <CardHeader>
              <CardDescription>{s.label}</CardDescription>
              <CardTitle className="type-headline-medium">{s.value}</CardTitle>
            </CardHeader>
          </Card>
        ))}
      </section>

      <section aria-label="Points clés" className="mt-8 grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {site.cards.map(c => (
          <Card key={c.title}>
            <CardHeader>
              <Badge variant="secondary">{c.tag}</Badge>
              <CardTitle className="type-title-large">{c.title}</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="type-body-medium text-on-surface-variant">{c.body}</p>
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
        <h2 className="type-headline-small">Questions fréquentes</h2>
        <Accordion type="single" collapsible className="mt-4">
          {site.faq.map((f, i) => (
            <AccordionItem key={f.id} value={f.id}>
              <AccordionTrigger data-bench-click={i === 0 ? "" : undefined}>{f.q}</AccordionTrigger>
              <AccordionContent>{f.a}</AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>
      </section>
    </Shell>
  );
}

createRoot(document.getElementById("root")!).render(<Home />);

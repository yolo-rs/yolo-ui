import { createRoot } from "react-dom/client";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@kit/components/accordion";
import { Separator } from "@kit/components/separator";
import { legalSections, site } from "../../shared/content";
import { Shell } from "./shell";

function Legal() {
  const l = site.legal;
  const sections = legalSections();
  return (
    <Shell active="legal">
      <article className="mx-auto max-w-3xl">
        <h1 className="type-headline-large">{l.title}</h1>
        <p className="mt-1 type-body-small text-on-surface-variant">{l.updated}</p>
        <p className="mt-4 type-body-large">{l.intro}</p>
        <Accordion type="single" collapsible className="mt-6">
          <AccordionItem value="toc">
            <AccordionTrigger data-bench-click="">{l.tocLabel}</AccordionTrigger>
            <AccordionContent>
              <ol className="grid gap-1 type-body-medium">
                {sections.map(s => (
                  <li key={s.id}>
                    <a className="text-primary underline-offset-4 hover:underline" href={`#${s.id}`}>
                      {s.title}
                    </a>
                  </li>
                ))}
              </ol>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
        <Separator className="my-6" />
        {sections.map(s => (
          <section key={s.id} id={s.id} className="mt-8">
            <h2 className="type-title-large text-primary">{s.title}</h2>
            {s.paragraphs.map((p, i) => (
              <p key={i} className="mt-3 type-body-large">
                {p}
              </p>
            ))}
          </section>
        ))}
      </article>
    </Shell>
  );
}

createRoot(document.getElementById("root")!).render(<Legal />);

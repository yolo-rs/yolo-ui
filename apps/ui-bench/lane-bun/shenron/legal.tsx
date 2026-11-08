import { createRoot } from "react-dom/client";
import { Accordion, AccordionItem, AccordionPanel, AccordionTrigger } from "@kit/components/accordion";
import { Separator } from "@kit/components/separator";
import { legalSections, site } from "../../shared/content";
import { Shell } from "./shell";

function Legal() {
  const l = site.legal;
  const sections = legalSections();
  return (
    <Shell active="legal">
      <article className="mx-auto max-w-3xl">
        <h1 className="font-title text-3xl font-extrabold">{l.title}</h1>
        <p className="mt-1 text-xs text-muted-foreground">{l.updated}</p>
        <p className="mt-4 text-lg">{l.intro}</p>
        <Accordion className="mt-6">
          <AccordionItem value="toc">
            <AccordionTrigger data-bench-click="">{l.tocLabel}</AccordionTrigger>
            <AccordionPanel>
              <ol className="grid gap-1 text-sm">
                {sections.map(s => (
                  <li key={s.id}>
                    <a className="text-dbz-orange underline-offset-4 hover:underline" href={`#${s.id}`}>
                      {s.title}
                    </a>
                  </li>
                ))}
              </ol>
            </AccordionPanel>
          </AccordionItem>
        </Accordion>
        <Separator className="my-6" />
        {sections.map(s => (
          <section key={s.id} id={s.id} className="mt-8">
            <h2 className="font-title text-xl font-extrabold text-dbz-orange">{s.title}</h2>
            {s.paragraphs.map((p, i) => (
              <p key={i} className="mt-3 text-base leading-relaxed">
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

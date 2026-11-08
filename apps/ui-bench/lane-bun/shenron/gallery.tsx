import { createRoot } from "react-dom/client";
import { Accordion, AccordionItem, AccordionPanel, AccordionTrigger } from "@kit/components/accordion";
import { Avatar, AvatarFallback } from "@kit/components/avatar";
import { Badge } from "@kit/components/badge";
import { Button, buttonVariants } from "@kit/components/button";
import { Card, CardContent, CardHeader, CardTitle } from "@kit/components/card";
import { Checkbox } from "@kit/components/checkbox";
import {
  Dialog,
  DialogClose,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogPopup,
  DialogTitle,
  DialogTrigger,
} from "@kit/components/dialog";
import { Input } from "@kit/components/input";
import { Label } from "@kit/components/label";
import { Progress } from "@kit/components/progress";
import { Select, SelectItem, SelectPopup, SelectTrigger, SelectValue } from "@kit/components/select";
import { Slider } from "@kit/components/slider";
import { Switch } from "@kit/components/switch";
import { Tabs, TabsList, TabsPanel, TabsTab } from "@kit/components/tabs";
import { site } from "../../shared/content";
import { Shell } from "./shell";

const BUTTON_VARIANTS = ["default", "secondary", "outline", "ghost", "destructive"] as const;
const BADGE_VARIANTS = ["default", "secondary", "success", "outline"] as const;

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-wrap items-center gap-3">{children}</CardContent>
    </Card>
  );
}

function Gallery() {
  const g = site.gallery;
  const items = g.options.map(o => ({ value: o, label: o }));
  return (
    <Shell active="gallery">
      <h1 className="font-title text-3xl font-extrabold">{g.title}</h1>
      <p className="mt-2 text-lg text-muted-foreground">{g.lead}</p>
      <div className="mt-6 grid gap-4 lg:grid-cols-2">
        <Section title="Boutons">
          {g.buttons.map((label, i) => (
            <Button key={label} variant={BUTTON_VARIANTS[i]}>
              {label}
            </Button>
          ))}
        </Section>
        <Section title="Badges">
          {g.badges.map((label, i) => (
            <Badge key={label} variant={BADGE_VARIANTS[i]}>
              {label}
            </Badge>
          ))}
        </Section>
        <Section title="Champs">
          <div className="grid w-full gap-2">
            <Label htmlFor="sh-email">Adresse électronique</Label>
            <Input id="sh-email" type="email" placeholder="nom@exemple.fr" />
          </div>
          <div className="grid w-full gap-2">
            <Label>{g.selectLabel}</Label>
            <Select defaultValue={g.options[0]} items={items}>
              <SelectTrigger className="w-full" aria-label={g.selectLabel}>
                <SelectValue />
              </SelectTrigger>
              <SelectPopup>
                {items.map(o => (
                  <SelectItem key={o.value} value={o.value}>
                    {o.label}
                  </SelectItem>
                ))}
              </SelectPopup>
            </Select>
          </div>
        </Section>
        <Section title="Choix">
          <Label className="flex items-center gap-2">
            <Checkbox defaultChecked /> Recevoir le rapport
          </Label>
          <Label className="flex items-center gap-2">
            <Switch defaultChecked /> Mode compact
          </Label>
          <div className="w-full">
            <Slider defaultValue={40} max={100} step={1} aria-label="Volume" />
          </div>
        </Section>
        <Section title="Onglets">
          <Tabs defaultValue={g.tabs[0]!.id} className="w-full">
            <TabsList>
              {g.tabs.map((t, i) => (
                <TabsTab key={t.id} value={t.id} data-bench-click={i === 1 ? "" : undefined}>
                  {t.label}
                </TabsTab>
              ))}
            </TabsList>
            {g.tabs.map(t => (
              <TabsPanel key={t.id} value={t.id} className="pt-3 text-sm">
                {t.body}
              </TabsPanel>
            ))}
          </Tabs>
        </Section>
        <Section title="Accordéon">
          <Accordion className="grid w-full gap-3">
            {site.faq.map(f => (
              <AccordionItem key={f.id} value={f.id}>
                <AccordionTrigger>{f.q}</AccordionTrigger>
                <AccordionPanel>{f.a}</AccordionPanel>
              </AccordionItem>
            ))}
          </Accordion>
        </Section>
        <Section title="Dialogue et alerte">
          <Dialog>
            <DialogTrigger className={buttonVariants({ variant: "default" })}>{g.dialog.trigger}</DialogTrigger>
            <DialogPopup closeLabel="Fermer">
              <DialogHeader>
                <DialogTitle>{g.dialog.title}</DialogTitle>
                <DialogDescription>{g.dialog.body}</DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose className={buttonVariants({ variant: "outline" })}>{g.dialog.cancel}</DialogClose>
                <DialogClose className={buttonVariants({ variant: "default" })}>{g.dialog.confirm}</DialogClose>
              </DialogFooter>
            </DialogPopup>
          </Dialog>
          <div role="status" className="card w-full p-4">
            <p className="font-bold">{g.dialog.title}</p>
            <p className="text-sm text-muted-foreground">{g.dialog.body}</p>
          </div>
        </Section>
        <Section title="Progression et avatars">
          <Progress value={g.progress} className="w-full" aria-label="Progression" />
          {g.people.map(p => (
            <Avatar key={p.initials} title={p.name}>
              <AvatarFallback>{p.initials}</AvatarFallback>
            </Avatar>
          ))}
        </Section>
      </div>
      <Card className="mt-4">
        <CardHeader>
          <CardTitle>Tableau</CardTitle>
        </CardHeader>
        <CardContent>
          <table className="w-full text-left text-sm">
            <thead className="border-b-2 border-black text-xs uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="py-2">Page</th>
                <th className="py-2">Voie</th>
                <th className="py-2">Statut</th>
                <th className="py-2 text-right">Score</th>
              </tr>
            </thead>
            <tbody>
              {g.table.map(r => (
                <tr key={`${r.name}-${r.lane}`} className="border-b border-[var(--border)]">
                  <td className="py-2">{r.name}</td>
                  <td className="py-2">{r.lane}</td>
                  <td className="py-2">
                    <Badge variant="outline">{r.status}</Badge>
                  </td>
                  <td className="py-2 text-right">{r.score}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </CardContent>
      </Card>
    </Shell>
  );
}

createRoot(document.getElementById("root")!).render(<Gallery />);

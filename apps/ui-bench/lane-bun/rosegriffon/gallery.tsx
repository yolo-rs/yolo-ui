import { createRoot } from "react-dom/client";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@kit/components/accordion";
import { Alert, AlertDescription, AlertTitle } from "@kit/components/alert";
import { Avatar, AvatarFallback } from "@kit/components/avatar";
import { Badge } from "@kit/components/badge";
import { Button, buttonVariants } from "@kit/components/button";
import { Card, CardContent, CardHeader, CardTitle } from "@kit/components/card";
import { Checkbox } from "@kit/components/checkbox";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@kit/components/dialog";
import { Input } from "@kit/components/input";
import { Label } from "@kit/components/label";
import { Progress } from "@kit/components/progress";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@kit/components/select";
import { Slider } from "@kit/components/slider";
import { Switch } from "@kit/components/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@kit/components/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@kit/components/tabs";
import { site } from "../../shared/content";
import { Shell } from "./shell";

const BUTTON_VARIANTS = ["default", "secondary", "outline", "ghost", "destructive"] as const;
const BADGE_VARIANTS = ["default", "secondary", "success", "outline"] as const;

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="type-title-medium">{title}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-wrap items-center gap-3">{children}</CardContent>
    </Card>
  );
}

function Gallery() {
  const g = site.gallery;
  return (
    <Shell active="gallery">
      <h1 className="type-headline-large">{g.title}</h1>
      <p className="mt-2 type-body-large text-on-surface-variant">{g.lead}</p>
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
            <Label htmlFor="rg-email">Adresse électronique</Label>
            <Input id="rg-email" type="email" placeholder="nom@exemple.fr" />
          </div>
          <div className="grid w-full gap-2">
            <Label>{g.selectLabel}</Label>
            <Select defaultValue={g.options[0]}>
              <SelectTrigger className="w-full" aria-label={g.selectLabel}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {g.options.map(o => (
                  <SelectItem key={o} value={o}>
                    {o}
                  </SelectItem>
                ))}
              </SelectContent>
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
            <Slider defaultValue={[40]} max={100} step={1} aria-label="Volume" />
          </div>
        </Section>
        <Section title="Onglets">
          <Tabs defaultValue={g.tabs[0]!.id} className="w-full">
            <TabsList>
              {g.tabs.map((t, i) => (
                <TabsTrigger key={t.id} value={t.id} data-bench-click={i === 1 ? "" : undefined}>
                  {t.label}
                </TabsTrigger>
              ))}
            </TabsList>
            {g.tabs.map(t => (
              <TabsContent key={t.id} value={t.id} className="pt-3 type-body-medium">
                {t.body}
              </TabsContent>
            ))}
          </Tabs>
        </Section>
        <Section title="Accordéon">
          <Accordion type="single" collapsible className="w-full">
            {site.faq.map(f => (
              <AccordionItem key={f.id} value={f.id}>
                <AccordionTrigger>{f.q}</AccordionTrigger>
                <AccordionContent>{f.a}</AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </Section>
        <Section title="Dialogue et alerte">
          <Dialog>
            <DialogTrigger className={buttonVariants({ variant: "default" })}>{g.dialog.trigger}</DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>{g.dialog.title}</DialogTitle>
                <DialogDescription>{g.dialog.body}</DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose className={buttonVariants({ variant: "outline" })}>{g.dialog.cancel}</DialogClose>
                <DialogClose className={buttonVariants({ variant: "default" })}>{g.dialog.confirm}</DialogClose>
              </DialogFooter>
            </DialogContent>
          </Dialog>
          <Alert variant="success" className="w-full">
            <AlertTitle>{g.dialog.title}</AlertTitle>
            <AlertDescription>{g.dialog.body}</AlertDescription>
          </Alert>
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
          <CardTitle className="type-title-medium">Tableau</CardTitle>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Page</TableHead>
                <TableHead>Voie</TableHead>
                <TableHead>Statut</TableHead>
                <TableHead className="text-right">Score</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {g.table.map(r => (
                <TableRow key={`${r.name}-${r.lane}`}>
                  <TableCell>{r.name}</TableCell>
                  <TableCell>{r.lane}</TableCell>
                  <TableCell>
                    <Badge variant="outline">{r.status}</Badge>
                  </TableCell>
                  <TableCell className="text-right">{r.score}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </Shell>
  );
}

createRoot(document.getElementById("root")!).render(<Gallery />);

import Image from "next/image";
import Link from "next/link";
import { alias } from "#alias-target";
import { widget } from "@ui/widget";
import note from "../ui/note.txt";

export default function Home() {
  return (
    <main>
      <h1>Home {process.env.APP_NAME}</h1>
      <p data-alias={alias} data-widget={widget} data-note={note}>config</p>
      <Image src="/photo.png" alt="photo" width={640} height={480} sizes="100vw" />
      <Link href="/about">About</Link>
    </main>
  );
}

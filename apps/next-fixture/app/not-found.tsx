import Link from "next/link";

export default function NotFound() {
  return (
    <main>
      <h2>Not found</h2>
      <p>The requested resource does not exist.</p>
      <Link href="/">Back home</Link>
    </main>
  );
}

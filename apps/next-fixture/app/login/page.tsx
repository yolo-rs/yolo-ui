import Link from "next/link";

export const metadata = { title: "Login" };

export default function LoginPage() {
  return (
    <main>
      <h1>Login</h1>
      <form method="post" action="/api/session">
        <input name="email" type="email" autoComplete="email" />
        <input name="password" type="password" autoComplete="current-password" />
        <button type="submit">Sign in</button>
      </form>
      <Link href="/">Back</Link>
    </main>
  );
}

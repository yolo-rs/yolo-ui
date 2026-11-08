export default async function UserPage({ params }: { params: Promise<{ username: string }> }) {
  const { username } = await params;
  return (
    <main>
      <h1>{username}</h1>
      <p>Rewritten from /u/{username}</p>
    </main>
  );
}

export const maxDuration = 30;

export default async function Dashboard() {
  async function refreshNow() {
    "use server";
    const { refresh } = await import("next/cache");
    refresh();
  }
  return (
    <form action={refreshNow}>
      <button>Refresh</button>
    </form>
  );
}

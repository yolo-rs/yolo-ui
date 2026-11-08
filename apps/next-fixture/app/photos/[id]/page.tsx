import Image from "next/image";

export default async function PhotoPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return (
    <main>
      <h1>Photo {id}</h1>
      <Image
        src={`https://cdn.example.com/media/${id}.jpg`}
        alt={`Photo ${id}`}
        width={1200}
        height={800}
      />
    </main>
  );
}

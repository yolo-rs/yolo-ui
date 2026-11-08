"use client";

import { use } from "react";
import { useRouter } from "next/navigation";

export default function PhotoModal({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const router = useRouter();
  return (
    <dialog open data-modal>
      <p>Photo {id}</p>
      <button type="button" onClick={() => router.back()}>
        Close
      </button>
    </dialog>
  );
}

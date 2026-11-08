"use client";

import { useActionState, useOptimistic, useState, useTransition } from "react";
import { useRouter, usePathname, useSearchParams } from "next/navigation";
import { createPost } from "../actions";

export function Counter({ initial }: { initial: number }) {
  const [count, setCount] = useState(initial);
  const [state, formAction, pending] = useActionState(createPost, { message: "" });
  const [optimistic, addOptimistic] = useOptimistic(count, (c: number, delta: number) => c + delta);
  const [isPending, startTransition] = useTransition();
  const router = useRouter();
  const pathname = usePathname();
  const params = useSearchParams();
  return (
    <form action={formAction}>
      <input name="title" />
      <button type="submit" disabled={pending}>
        Create
      </button>
      <button
        type="button"
        onClick={() =>
          startTransition(() => {
            addOptimistic(1);
            setCount((c) => c + 1);
            router.refresh();
          })
        }
      >
        {optimistic}
      </button>
      <p>
        {state.message} {pathname} {params.get("q")} {isPending ? "pending" : ""}
      </p>
    </form>
  );
}

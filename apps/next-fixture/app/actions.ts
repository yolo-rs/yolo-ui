"use server";

import { revalidatePath, revalidateTag, updateTag } from "next/cache";
import { cookies, headers } from "next/headers";
import { redirect } from "next/navigation";

export async function createPost(prevState: { message: string }, formData: FormData) {
  const title = String(formData.get("title") ?? "");
  const h = await headers();
  const c = await cookies();
  c.set("last-title", title, { httpOnly: true, sameSite: "lax" });
  updateTag("posts");
  revalidatePath("/");
  return { message: `created ${title} from ${h.get("user-agent") ?? "?"}` };
}

export async function deletePost(slug: string) {
  revalidateTag("posts", "max");
  redirect("/");
}

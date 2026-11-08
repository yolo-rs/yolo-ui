// SPDX-License-Identifier: Apache-2.0
// next/cache shim.

/** Cache functions of `next/cache` (Next.js 16 names plus the `unstable_*` aliases). */
export {
  revalidatePath,
  revalidateTag,
  updateTag,
  refresh,
  cacheTag,
  cacheLife,
  cacheTag as unstable_cacheTag,
  cacheLife as unstable_cacheLife,
  unstable_cache,
  unstable_noStore,
  unstable_expireTag,
  unstable_expirePath,
  io,
} from "../cache.ts";

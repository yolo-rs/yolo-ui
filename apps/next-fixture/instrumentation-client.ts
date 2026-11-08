export function onRouterTransitionStart(
  url: string,
  navigationType: "push" | "replace" | "traverse",
) {
  performance.mark(`nav-${navigationType}-${url}`);
}

export async function register() {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    console.log("register");
  }
}

export async function onRequestError(err: unknown, request: { path: string }) {
  console.error(err, request.path);
}

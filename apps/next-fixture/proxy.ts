import { NextResponse, type NextRequest } from "next/server";

export function proxy(request: NextRequest) {
  const token = request.cookies.get("session")?.value;
  if (!token && request.nextUrl.pathname.startsWith("/dashboard")) {
    return NextResponse.redirect(new URL("/login", request.url));
  }
  if (request.nextUrl.pathname.startsWith("/u/")) {
    return NextResponse.rewrite(
      new URL(request.nextUrl.pathname.replace("/u/", "/users/"), request.url),
    );
  }
  const response = NextResponse.next({ request: { headers: new Headers(request.headers) } });
  response.headers.set("x-yolo-proxy", "1");
  return response;
}

export const config = {
  matcher: [
    "/dashboard/:path*",
    "/u/:path*",
    { source: "/api/:path*", missing: [{ type: "header", key: "x-internal" }] },
  ],
};

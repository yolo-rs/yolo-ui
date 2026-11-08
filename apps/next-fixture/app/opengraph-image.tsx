import { ImageResponse } from "next/og";

export const alt = "Yolo Fixture";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default async function Image() {
  return new ImageResponse(
    (
      <div
        style={{
          fontSize: 64,
          background: "#09090b",
          color: "#fafafa",
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        Yolo Fixture
      </div>
    ),
    { ...size },
  );
}

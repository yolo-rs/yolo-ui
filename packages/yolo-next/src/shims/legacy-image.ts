// SPDX-License-Identifier: Apache-2.0
// next/legacy/image shim: maps layout / objectFit / objectPosition onto the modern Image component.

import { createElement, type CSSProperties, type ReactElement } from "react";
import { Image, type ImageProps, type LegacyLayout } from "../image.ts";

/** Props of the legacy `next/legacy/image` component. */
export interface LegacyImageProps extends ImageProps {
  readonly layout?: LegacyLayout;
  readonly objectFit?: CSSProperties["objectFit"];
  readonly objectPosition?: CSSProperties["objectPosition"];
  readonly lazyBoundary?: string;
  readonly lazyRoot?: unknown;
}

/** Legacy `Image`: `layout="fill"` becomes `fill`, `objectFit` / `objectPosition` move into `style`. */
export default function LegacyImage(props: LegacyImageProps): ReactElement {
  const { layout, objectFit, objectPosition, lazyBoundary: _lazyBoundary, lazyRoot: _lazyRoot, style, ...rest } = props;
  const fill = layout === "fill" || rest.fill === true;
  const mergedStyle: CSSProperties = {
    ...(fill ? {} : { objectFit, objectPosition }),
    ...style,
  };
  const mapped: ImageProps = {
    ...rest,
    fill,
    style: mergedStyle,
    ...(layout && layout !== "fill" ? { layout } : {}),
    ...(fill ? { objectFit, objectPosition } : {}),
  };
  return createElement(Image, mapped);
}

/** Legacy `ImageProps` alias. */
export type { ImageProps };

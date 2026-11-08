// SPDX-License-Identifier: Apache-2.0
// next/form shim: GET forms navigate client-side with the fields as search params; function actions pass through.

import {
  createElement,
  type FormHTMLAttributes,
  type MouseEvent,
  type ReactElement,
  type ReactNode,
  type SubmitEvent as ReactSubmitEvent,
} from "react";
import { useYoloRouter } from "../navigation.ts";

/** Props of `Form` (`next/form`). */
export interface FormProps extends Omit<FormHTMLAttributes<HTMLFormElement>, "action" | "method" | "encType" | "target"> {
  readonly action: string | ((formData: FormData) => void | Promise<void>);
  readonly replace?: boolean;
  readonly scroll?: boolean;
  readonly prefetch?: boolean | null;
  readonly children?: ReactNode;
}

/** Builds the GET navigation URL of a form: `action` with its query replaced by the form fields. */
export function formTargetUrl(action: string, formData: FormData, base = "http://yolo.local"): string {
  const url = new URL(action, base);
  const params = new URLSearchParams();
  for (const [name, value] of formData) {
    if (typeof value === "string") {
      params.append(name, value);
    }
  }
  url.search = params.toString();
  return `${url.pathname}${url.search}${url.hash}`;
}

/** `next/form`: string actions become client-side GET navigations, function actions are passed to React. */
export default function Form(props: FormProps): ReactElement {
  const { action, replace = false, scroll = true, prefetch = null, onSubmit, onMouseEnter, children, ...rest } = props;
  const router = useYoloRouter();
  if (typeof action === "function") {
    return createElement("form", { ...rest, action, onSubmit, onMouseEnter }, children);
  }
  const handleSubmit = (event: ReactSubmitEvent<HTMLFormElement>): void => {
    onSubmit?.(event);
    if (event.defaultPrevented) {
      return;
    }
    const submitter = event.nativeEvent.submitter;
    if (submitter && (submitter.hasAttribute("formaction") || submitter.hasAttribute("formmethod") || submitter.hasAttribute("formtarget"))) {
      return;
    }
    event.preventDefault();
    const target = formTargetUrl(action, new FormData(event.currentTarget), window.location.href);
    void router.navigate(target, { replace, scroll });
  };
  return createElement(
    "form",
    {
      ...rest,
      action,
      method: "get",
      onSubmit: handleSubmit,
      onMouseEnter: (event: MouseEvent<HTMLFormElement>) => {
        onMouseEnter?.(event);
        if (prefetch !== false) {
          router.prefetch(action);
        }
      },
    },
    children,
  );
}

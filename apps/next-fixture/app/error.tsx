"use client";

export default function Error({
  error,
  reset,
  retry,
}: {
  error: Error & { digest?: string };
  reset: () => void;
  retry: () => void;
}) {
  return (
    <div role="alert">
      <h2>Something went wrong</h2>
      <p>{error.message}</p>
      {error.digest ? <code>{error.digest}</code> : null}
      <button type="button" onClick={() => reset()}>
        Reset
      </button>
      <button type="button" onClick={() => retry()}>
        Retry
      </button>
    </div>
  );
}

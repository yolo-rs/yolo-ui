/**
 * The source offer of the web build (AGPL-3.0, section 13).
 *
 * Whoever lets people use a modified version of this program over a network
 * has to offer them its corresponding source, from the program itself. The
 * desktop app is handed to the user as a package with its source a release
 * away; the web build is used over the network, so it says where its source
 * is, on every screen (the footer) and in Settings next to the version.
 *
 * Where: the build constant VITE_LU_SOURCE_URL (the Yolo fork when nobody
 * set one), pinned to VITE_LU_SOURCE_COMMIT when the build names its commit
 * (lib/build-target.ts). Whoever deploys a changed build points the first at
 * the repository that holds the change and sets the second to its commit.
 *
 * Renders nothing outside the web build. The notices the app already carries
 * (LICENSE, the package's license field, the upstream credits) stay as they
 * are; this adds to them.
 */
import { useEffect, useState } from 'react'
import { isWebBuild, sourceCommit, sourceLink } from '../../lib/build-target'
import { webBackupHasFileSystem } from '../../lib/web-backup'
import { backupStoresNow } from '../../lib/store-backup'

export const AGPL_URL = 'https://www.gnu.org/licenses/agpl-3.0.html'

/** "Source code", with the short commit when the build named one. */
export function SourceCodeLink({ className = '' }: { className?: string }) {
  const commit = sourceCommit()
  return (
    <a
      href={sourceLink()}
      target="_blank"
      rel="noopener noreferrer"
      className={`text-lu-accent hover:text-lu-accent-hover underline-offset-2 hover:underline transition-colors ${className}`}
    >
      Source code{commit ? <span className="font-mono"> ({commit.slice(0, 12)})</span> : null}
    </a>
  )
}

/** The row in Settings, under the version. */
export function SourceOfferRow() {
  if (!isWebBuild()) return null
  return (
    <div className="flex items-center justify-between">
      <span className="text-[0.65rem] text-gray-500">
        Free software under the{' '}
        <a href={AGPL_URL} target="_blank" rel="noopener noreferrer" className="underline underline-offset-2 hover:text-gray-300">GNU AGPL-3.0</a>
      </span>
      <SourceCodeLink className="text-[0.65rem]" />
    </div>
  )
}

/** The footer of the web build. Also carries the backup download where the
 *  browser has no origin private file system to keep the backup in. */
export function WebSourceFooter() {
  const [noFileSystem, setNoFileSystem] = useState(false)
  useEffect(() => {
    if (!isWebBuild()) return
    let cancelled = false
    void webBackupHasFileSystem().then((has) => { if (!cancelled) setNoFileSystem(!has) }, () => {})
    return () => { cancelled = true }
  }, [])
  if (!isWebBuild()) return null
  return (
    <footer className="shrink-0 flex items-center justify-center gap-2 px-3 pb-1.5 text-[0.6rem] text-gray-500 dark:text-gray-500">
      <span>
        Locally Uncensored is free software under the{' '}
        <a href={AGPL_URL} target="_blank" rel="noopener noreferrer" className="underline underline-offset-2 hover:text-gray-300">GNU AGPL-3.0</a>.
      </span>
      <SourceCodeLink />
      {noFileSystem && (
        <>
          <span aria-hidden="true">·</span>
          <button
            type="button"
            onClick={() => { void backupStoresNow() }}
            title="This browser has no private file system for the automatic backup. Save a copy of your chats and settings as a file."
            className="text-lu-accent hover:text-lu-accent-hover transition-colors"
          >
            Download backup
          </button>
        </>
      )}
    </footer>
  )
}

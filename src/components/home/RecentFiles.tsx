import { Link } from '@tanstack/react-router'
import { Icon } from '@/components/ui/Icon.tsx'
import { IconButton } from '@/components/ui/IconButton.tsx'
import type { StoredFile } from '@/services/DocumentLibrary.ts'
import { useLibraryStore } from '@/stores/libraryStore.ts'

const dates = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' })
const times = new Intl.DateTimeFormat(undefined, { timeStyle: 'short' })
const sizes = new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 })

function when(moment: number): string {
  return new Date(moment).toDateString() === new Date().toDateString() ? times.format(moment) : dates.format(moment)
}

function size(bytes: number): string {
  if (bytes < 1024 * 1024) return `${sizes.format(Math.max(1, Math.round(bytes / 1024)))} KB`
  return `${sizes.format(bytes / (1024 * 1024))} MB`
}

export function RecentFiles({ files }: { files: StoredFile[] }) {
  const askToDelete = useLibraryStore((state) => state.askToDelete)
  return (
    <section className="recent-files" aria-labelledby="recent-title">
      <h2 id="recent-title" className="recent-title">
        Recent
      </h2>
      <table className="recent-table">
        <thead>
          <tr>
            <th scope="col">Name</th>
            <th scope="col">Opened</th>
            <th scope="col">Size</th>
            <th scope="col">
              <span className="visually-hidden">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {files.map((file) => (
            <tr key={file.id}>
              <td>
                <Link className="recent-name" to="/documents/$documentId" params={{ documentId: file.id }}>
                  <span className="recent-icon">
                    <Icon name="file-text" />
                  </span>
                  <span>
                    <span className="recent-file">{file.name}</span>
                    <span className="recent-kind">PDF</span>
                  </span>
                </Link>
              </td>
              <td>{when(file.opened)}</td>
              <td>{size(file.size)}</td>
              <td className="recent-actions">
                <IconButton icon="delete" label={`Delete ${file.name}`} tooltip="left" onClick={() => askToDelete(file)} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}

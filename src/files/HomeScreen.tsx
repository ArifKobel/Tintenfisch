import { OpenDocumentButton } from '@document/OpenDocumentButton.tsx'
import { DeleteFileDialog } from '@files/DeleteFileDialog.tsx'
import { RecentFiles } from '@files/RecentFiles.tsx'
import { SampleDocumentButton } from '@files/SampleDocumentButton.tsx'
import { useFilesState } from '@files/useFilesState.ts'

export function HomeScreen() {
  const { files, missing } = useFilesState()
  return (
    <div className="home">
      <section className="start-card">
        <h1 className="start-title">Drop a PDF here or select one</h1>
        <p className="start-text">Edit text and images, comment, organize pages, fill and sign forms. Files stay in this browser and never leave your device.</p>
        <div className="start-actions">
          <OpenDocumentButton />
          <SampleDocumentButton />
        </div>
      </section>
      {missing && (
        <p className="home-notice" role="status">
          That document is not stored in this browser. It may have been deleted or opened on another device.
        </p>
      )}
      {files.length > 0 && <RecentFiles files={files} />}
      <DeleteFileDialog />
    </div>
  )
}

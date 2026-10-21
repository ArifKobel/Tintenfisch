import { OpenDocumentButton } from '@/components/document/OpenDocumentButton.tsx'
import { DeleteFileDialog } from '@/components/home/DeleteFileDialog.tsx'
import { RecentFiles } from '@/components/home/RecentFiles.tsx'
import { SampleDocumentButton } from '@/components/home/SampleDocumentButton.tsx'
import { useLibraryStore } from '@/stores/libraryStore.ts'

export function HomeScreen() {
  const { files, missing } = useLibraryStore()
  return (
    <div className="flex size-full flex-col items-center gap-8 overflow-y-auto px-6 py-12 max-md:gap-6 max-md:px-4 max-md:py-6">
      <section className="flex w-[min(100%,960px)] flex-none flex-col items-center gap-4 rounded-lg border border-dashed border-field-border bg-surface px-14 py-10 text-center max-md:px-5 max-md:py-7">
        <h1 className="text-[22px] leading-7 font-bold text-ink-strong">Drop a PDF here or select one</h1>
        <p className="max-w-[520px] text-[15px] text-ink-body">Edit text and images, comment, organize pages, fill and sign forms. Files stay in this browser and never leave your device.</p>
        <div className="flex flex-col items-center gap-2">
          <OpenDocumentButton />
          <SampleDocumentButton />
        </div>
      </section>
      {missing && (
        <p className="w-[min(100%,960px)] rounded bg-surface px-4 py-2.5 text-ink-strong shadow-[inset_4px_0_0_var(--color-accent)]" role="status">
          That document is not stored in this browser. It may have been deleted or opened on another device.
        </p>
      )}
      {files.length > 0 && <RecentFiles files={files} />}
      <DeleteFileDialog />
    </div>
  )
}

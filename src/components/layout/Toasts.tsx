import { useToastStore } from '@/stores/toastStore.ts'

const TOAST = 'pointer-events-auto rounded bg-negative px-4 py-2.5 text-white shadow-dialog'

export function Toasts() {
  const { toasts } = useToastStore()
  return (
    <div className="pointer-events-none absolute bottom-6 left-1/2 z-15 flex -translate-x-1/2 flex-col items-center gap-2 max-md:bottom-20 max-md:w-[calc(100%-32px)]" aria-live="polite">
      {toasts.editing && (
        <span className={TOAST} role="status">
          {toasts.editing}
        </span>
      )}
      {toasts.forms && (
        <span className={TOAST} role="status">
          {toasts.forms}
        </span>
      )}
      {toasts.library && (
        <span className={TOAST} role="alert">
          {toasts.library}
        </span>
      )}
    </div>
  )
}

import { useToastStore } from '@/stores/toastStore.ts'

export function Toasts() {
  const { toasts } = useToastStore()
  return (
    <div className="toasts" aria-live="polite">
      {toasts.editing && (
        <span className="toast" role="status">
          {toasts.editing}
        </span>
      )}
      {toasts.forms && (
        <span className="toast" role="status">
          {toasts.forms}
        </span>
      )}
      {toasts.library && (
        <span className="toast" role="alert">
          {toasts.library}
        </span>
      )}
    </div>
  )
}

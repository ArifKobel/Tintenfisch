import { create } from 'zustand'

export type ToastSource = 'editing' | 'forms' | 'library'

interface ToastStore {
  toasts: Readonly<Partial<Record<ToastSource, string>>>
  show(source: ToastSource, message: string): void
  clear(source: ToastSource): void
}

export const useToastStore = create<ToastStore>()((set, get) => ({
  toasts: {},

  show(source, message) {
    set({ toasts: { ...get().toasts, [source]: message } })
  },

  clear(source) {
    if (!(source in get().toasts)) return
    const { [source]: _cleared, ...toasts } = get().toasts
    set({ toasts })
  },
}))

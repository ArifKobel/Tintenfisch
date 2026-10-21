import { useEffect, useRef } from 'react'
import { Button } from '@/components/ui/Button.tsx'
import { Dialog, DialogActions } from '@/components/ui/Dialog.tsx'
import { Hint } from '@/components/ui/Text.tsx'
import { useFontStore } from '@/stores/fontStore.ts'

export function FontConsentDialog() {
  const { asking, downloads, answer } = useFontStore()
  const allow = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    if (!asking) return
    const previous = document.activeElement
    allow.current?.focus()
    return () => {
      if (previous instanceof HTMLElement) previous.focus({ preventScroll: true })
    }
  }, [asking])

  if (!asking) return null
  return (
    <Dialog title="Download fonts?" titleId="font-consent-title" role="alertdialog" keepsEdit className="max-w-[480px]">
        <p className="text-ink-body">
          PDFs often carry only the letters they already use. To type new ones in the original font, Tintenfisch can download it, or a similar open font, from Fontshare (cdn.fontshare.com, Indian Type Foundry) and Fontsource (cdn.jsdelivr.net, jsDelivr).
        </p>
        <p className="text-ink-body">Your browser then sends its IP address to these services. Your documents never leave this device. Without downloads, missing letters use a built-in font.</p>
        <Hint>You can change this later under More options.{downloads !== 'unknown' && ` Currently ${downloads === 'allowed' ? 'allowed' : 'not allowed'}.`}</Hint>
        <DialogActions>
          <Button onClick={() => answer(false)}>Don't allow</Button>
          <Button ref={allow} look="primary" onClick={() => answer(true)}>
            Allow downloads
          </Button>
        </DialogActions>
    </Dialog>
  )
}

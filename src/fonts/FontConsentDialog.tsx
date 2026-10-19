import { useEffect, useRef } from 'react'
import { useApplication } from '@app/ApplicationContext.ts'
import { useFontConsentState } from '@fonts/useFontConsentState.ts'

export function FontConsentDialog() {
  const { fontConsent } = useApplication()
  const { asking, downloads } = useFontConsentState()
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
    <div className="dialog-backdrop">
      <div className="dialog font-consent" role="alertdialog" aria-modal="true" aria-labelledby="font-consent-title">
        <h2 id="font-consent-title" className="dialog-title">
          Download fonts?
        </h2>
        <p className="dialog-hint">
          PDFs often carry only the letters they already use. To type new ones in the original font, Tintenfisch can download it, or a similar open font, from Fontshare (cdn.fontshare.com, Indian Type Foundry) and Fontsource (cdn.jsdelivr.net, jsDelivr).
        </p>
        <p className="dialog-hint">Your browser then sends its IP address to these services. Your documents never leave this device. Without downloads, missing letters use a built-in font.</p>
        <p className="hint">You can change this later under More options.{downloads !== 'unknown' && ` Currently ${downloads === 'allowed' ? 'allowed' : 'not allowed'}.`}</p>
        <div className="dialog-actions">
          <button type="button" className="button" onClick={() => fontConsent.answer(false)}>
            Don't allow
          </button>
          <button ref={allow} type="button" className="button button-primary" onClick={() => fontConsent.answer(true)}>
            Allow downloads
          </button>
        </div>
      </div>
    </div>
  )
}

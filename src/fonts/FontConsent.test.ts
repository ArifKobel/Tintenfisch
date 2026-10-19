import { describe, expect, it } from 'vitest'
import { FontConsent } from '@fonts/FontConsent.ts'

const memory = (stored: string | null) => {
  const values = new Map(stored ? [['tintenfisch.fontDownloads', stored]] : [])
  return { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => void values.set(key, value) }
}

describe('FontConsent', () => {
  it('answers at once when downloads are allowed', async () => {
    const consent = new FontConsent(memory('allowed'))
    expect(await consent.request()).toBe(true)
    expect(consent.getSnapshot().asking).toBe(false)
  })

  it('asks again on request even after a refusal and waits for the answer', async () => {
    const consent = new FontConsent(memory('refused'))
    const answer = consent.request()
    expect(consent.getSnapshot()).toEqual({ downloads: 'refused', asking: true })
    consent.answer(true)
    expect(await answer).toBe(true)
    expect(new FontConsent(memory('refused')).getSnapshot().downloads).toBe('refused')
  })
})

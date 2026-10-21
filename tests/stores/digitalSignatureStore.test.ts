import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { SignatureInfo } from '@/types/protocol.ts'
import { signedName, useDigitalSignatureStore } from '@/stores/digitalSignatureStore.ts'
import { useDocumentStore } from '@/stores/documentStore.ts'
import { FakeEnginePort } from '@tests/fakes/FakeEnginePort.ts'

const openedResult = { status: 'opened', version: '1.7', byteLength: 4, objectCount: 9, pageCount: 1, repaired: false }
const pages = [{ id: '3-0', width: 612, height: 792, rotate: 0 }]
const request = { fieldId: '70-0', certificate: new Uint8Array([48, 130]), password: 'secret', reason: 'Approved', location: '' }
const report: SignatureInfo = {
  fieldId: '70-0',
  fieldName: 'Signature',
  signer: 'Ada Lovelace',
  signedAt: 'D:20261018120000Z',
  reason: 'Approved',
  status: 'valid',
  problem: null,
  changedAfter: false,
}

async function createStore() {
  const port = FakeEnginePort.attach()
  const documents = useDocumentStore.getState()
  const opening = documents.open('contract.pdf', new Uint8Array(4))
  await port.answerNext(openedResult)
  await port.answerNext(pages)
  await opening
  const store = useDigitalSignatureStore.getState()
  return { port, documents, store }
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] })
  vi.setSystemTime(new Date(Date.UTC(2026, 9, 18, 12, 0, 0)))
})

describe('digitalSignatureStore', () => {
  it('signs with the date of now, reopens the signed file under a signed name and hands it back', async () => {
    const { port, store } = await createStore()
    store.openDialog()
    const signing = store.sign(request)
    expect(useDigitalSignatureStore.getState().signing).toBe(true)
    expect(port.requests.at(-1)).toMatchObject({ kind: 'signField', ...request, date: 'D:20261018120000Z' })
    await port.answerNext(new Uint8Array([37, 80, 68, 70]))
    expect(port.requests.at(-1)).toMatchObject({ kind: 'open' })
    await port.answerNext(openedResult)
    await port.answerNext(pages)
    const signed = await signing
    expect(signed?.name).toBe('contract-signed.pdf')
    expect(useDocumentStore.getState().document).toMatchObject({ phase: 'ready', name: 'contract-signed.pdf', session: 2 })
    expect(useDigitalSignatureStore.getState()).toMatchObject({ dialogOpen: false, signing: false })
    expect(signedName('contract-signed.pdf')).toBe('contract-signed.pdf')
  })

  it('keeps the dialog open with the reason when signing is refused, and lists checked signatures', async () => {
    const { port, store } = await createStore()
    store.openDialog()
    const signing = store.sign(request)
    await Promise.resolve()
    port.answer({ id: port.lastId, ok: false, error: 'the password does not open the certificate file' })
    expect(await signing).toBeNull()
    expect(useDigitalSignatureStore.getState()).toMatchObject({ dialogOpen: true, notice: 'the password does not open the certificate file' })
    const loading = store.reload()
    await port.answerNext([report])
    await loading
    expect(useDigitalSignatureStore.getState().reports).toEqual([report])
  })
})

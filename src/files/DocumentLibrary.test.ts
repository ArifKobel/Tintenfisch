import { IDBFactory } from 'fake-indexeddb'
import { describe, expect, it } from 'vitest'
import { DocumentLibrary } from '@files/DocumentLibrary.ts'

function createLibrary() {
  let clock = 1000
  const library = new DocumentLibrary(new IDBFactory(), () => clock)
  return { library, tick: () => (clock += 1000) }
}

describe('DocumentLibrary', () => {
  it('keeps added files with their bytes and lists the last opened first', async () => {
    const { library, tick } = createLibrary()
    const first = await library.add('first.pdf', new Uint8Array([1, 2, 3]))
    tick()
    const second = await library.add('second.pdf', new Uint8Array([4]))
    expect((await library.list()).map((file) => file.name)).toEqual(['second.pdf', 'first.pdf'])
    tick()
    const opened = await library.open(first.id)
    expect(opened?.bytes).toEqual(new Uint8Array([1, 2, 3]))
    expect((await library.list()).map((file) => file.id)).toEqual([first.id, second.id])
  })

  it('replaces the bytes of a file and notes when and how large', async () => {
    const { library, tick } = createLibrary()
    const file = await library.add('report.pdf', new Uint8Array([1]))
    tick()
    await library.replace(file.id, new Uint8Array([1, 2, 3, 4]))
    const [stored] = await library.list()
    expect(stored).toMatchObject({ size: 4, modified: 2000, opened: 1000 })
    expect((await library.open(file.id))?.bytes).toEqual(new Uint8Array([1, 2, 3, 4]))
  })

  it('forgets removed files and answers unknown ones with nothing', async () => {
    const { library } = createLibrary()
    const file = await library.add('gone.pdf', new Uint8Array([1]))
    await library.remove(file.id)
    expect(await library.list()).toEqual([])
    expect(await library.open(file.id)).toBeNull()
    await expect(library.replace(file.id, new Uint8Array([2]))).rejects.toThrow('no longer stored')
  })
})

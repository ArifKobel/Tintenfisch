import { describe, expect, it } from 'vitest'
import { EditSession } from '@/lib/EditSession.ts'

const line = () => EditSession.open('3-0', 2, ['H', 'e', 'l', 'l', 'o'], 5)

describe('EditSession', () => {
  it('inserts at the caret and moves the caret past the insertion', () => {
    const session = line().moveTo(1).insert('ey')
    expect(session.text).toBe('Heyello')
    expect(session.caret).toBe(3)
    expect(session.changed).toBe(true)
  })

  it('counts a combined character as one caret stop', () => {
    const session = line().insert('é!')
    expect(session.clusters.slice(5)).toEqual(['é', '!'])
    expect(session.deleteBackward().deleteBackward().text).toBe('Hello')
  })

  it('deletes on either side of the caret and stops at the ends', () => {
    expect(line().deleteBackward().text).toBe('Hell')
    expect(line().deleteForward()).toEqual(line())
    expect(line().moveTo(0).deleteBackward().text).toBe('Hello')
    expect(line().moveTo(0).deleteForward().text).toBe('ello')
  })

  it('keeps the caret inside the line', () => {
    expect(line().moveBy(4).caret).toBe(5)
    expect(line().moveTo(-3).caret).toBe(0)
    expect(line().moveTo(0).moveToEnd().caret).toBe(5)
  })

  it('counts a moved or resized box as a change and drafts it with the text', () => {
    expect(line().movedTo(3, 4).changed).toBe(true)
    expect(line().resizedTo(120).draft).toEqual({ clusters: ['H', 'e', 'l', 'l', 'o'], caret: 5, dx: 0, dy: 0, width: 120, styles: [], selection: null })
    expect(line().movedTo(3, 4).insert('!').box).toEqual({ dx: 3, dy: 4, width: null })
  })

  it('reads as unchanged once the text returns to the original', () => {
    expect(line().insert('!').deleteBackward().changed).toBe(false)
  })

  it('selects with an anchor and replaces or deletes the selection as a whole', () => {
    const selected = line().moveTo(1).extendBy(3)
    expect(selected.selection).toEqual([1, 4])
    expect(selected.insert('i').text).toBe('Hio')
    expect(selected.deleteBackward().text).toBe('Ho')
    expect(selected.deleteForward().caret).toBe(1)
    expect(selected.moveBy(1).selection).toBeNull()
    expect(line().moveTo(3).extendTo(1).selection).toEqual([1, 3])
    expect(line().selectAll().selection).toEqual([0, 5])
  })

  it('styles the selection and keeps the styled range on its text as the text around it changes', () => {
    const bold = line().moveTo(1).extendBy(2).formatted({ bold: true })
    expect(bold.styles).toEqual([{ start: 1, end: 3, bold: true }])
    expect(bold.look.bold).toBe(true)
    expect(bold.changed).toBe(true)
    expect(bold.moveTo(0).insert('>>').styles).toEqual([{ start: 3, end: 5, bold: true }])
    expect(bold.moveTo(3).insert('!').styles).toEqual([{ start: 1, end: 4, bold: true }])
    expect(bold.moveTo(4).insert('!').styles).toEqual([{ start: 1, end: 3, bold: true }])
    expect(bold.moveTo(2).deleteBackward().styles).toEqual([{ start: 1, end: 2, bold: true }])
    expect(bold.moveTo(0).extendTo(4).deleteForward().styles).toEqual([])
  })

  it('types in a style chosen at the caret until the caret moves', () => {
    const typed = line().moveTo(2).formatted({ italic: true }).insert('ab').insert('c')
    expect(typed.text).toBe('Heabcllo')
    expect(typed.styles).toEqual([{ start: 2, end: 5, italic: true }])
    expect(typed.look.italic).toBe(true)
    expect(typed.moveBy(1).insert('x').styles).toEqual([{ start: 2, end: 5, italic: true }])
  })

  it('reads the look of the text before the caret from the paragraph styles', () => {
    const red = { resource: 'F2', font: 'Times', size: 10, bold: false, italic: true, color: [1, 0, 0] as [number, number, number] }
    const plain = { ...red, resource: 'F1', italic: false, color: null }
    const session = EditSession.open('3-0', 0, ['a', 'b'], 1, [plain, red])
    expect(session.look).toEqual({ font: { kind: 'document', name: 'F1' }, size: 10, bold: false, italic: false, color: null })
    expect(session.moveToEnd().look.color).toEqual([1, 0, 0])
  })

  it('keeps the look of the paragraph once all its text is gone', () => {
    const heading = { resource: 'F5', font: 'Cabinet Grotesk Extrabold', size: 40, bold: true, italic: false, color: [0, 0, 0] as [number, number, number] }
    const emptied = EditSession.open('3-0', 0, ['a', 'b'], 2, [heading, heading]).deleteBackward().deleteBackward()
    expect(emptied.text).toBe('')
    expect(emptied.look).toEqual({ font: { kind: 'document', name: 'F5' }, size: 40, bold: true, italic: false, color: [0, 0, 0] })
    expect(emptied.insert('x').look.size).toBe(40)
  })

  it('selects the word or the run of spaces under a double click', () => {
    const session = EditSession.open('3-0', 0, [...'one two  three'], 0)
    expect(session.selectWord(5).selection).toEqual([4, 7])
    expect(session.selectWord(7).selection).toEqual([7, 9])
    expect(session.selectWord(14).selection).toEqual([9, 14])
  })

  it('formats new text as a whole', () => {
    const fresh = EditSession.openNew('3-0', [10, 20]).insert('Hi').formatted({ bold: true, size: 20 })
    expect(fresh.newLook).toMatchObject({ bold: true, size: 20, font: { kind: 'bundled', family: 'sans' } })
    expect(fresh.styles).toEqual([])
  })
})

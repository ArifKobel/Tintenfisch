import type { CSSProperties, KeyboardEvent } from 'react'
import type { FormField, FormWidget } from '@/types/protocol.ts'
import { useFormsStore } from '@/stores/formsStore.ts'

const TEXT = 'pointer-events-auto absolute m-0 disabled:cursor-default resize-none border border-transparent bg-transparent px-0.5 py-0 font-[Helvetica,Arial,sans-serif] text-transparent caret-transparent hover:border-selection/60 focus:bg-white focus:text-caret focus:caret-auto focus:outline-2 focus:outline-selection'

const TOGGLE = 'pointer-events-auto absolute m-0 disabled:cursor-default cursor-pointer opacity-0'

const FONT_SHARE = 0.6

export function FormControl({ field, widget, cssScale }: { field: FormField; widget: FormWidget; cssScale: number }) {
  const forms = useFormsStore()
  const style: CSSProperties = {
    left: widget.rect.x * cssScale,
    top: widget.rect.y * cssScale,
    width: widget.rect.width * cssScale,
    height: widget.rect.height * cssScale,
    fontSize: Math.min(widget.rect.height * FONT_SHARE, 12) * cssScale,
  }
  const label = field.name

  switch (field.kind) {
    case 'text': {
      const commit = (text: string) => void forms.fill(field.id, { kind: 'text', text })
      const leaveOnEnter = (event: KeyboardEvent<HTMLInputElement>) => {
        if (event.key === 'Enter') event.currentTarget.blur()
      }
      return field.multiline ? (
        <textarea
          key={field.text}
          className={TEXT}
          style={style}
          aria-label={label}
          defaultValue={field.text}
          disabled={field.readOnly}
          onBlur={(event) => commit(event.target.value)}
        />
      ) : (
        <input
          key={field.text}
          className={TEXT}
          style={style}
          aria-label={label}
          type={field.password ? 'password' : 'text'}
          maxLength={field.maxLength ?? undefined}
          defaultValue={field.text}
          disabled={field.readOnly}
          onKeyDown={leaveOnEnter}
          onBlur={(event) => commit(event.target.value)}
        />
      )
    }
    case 'checkbox':
      return (
        <input
          className={TOGGLE}
          style={style}
          aria-label={label}
          type="checkbox"
          checked={field.on !== null}
          disabled={field.readOnly}
          onChange={(event) => void forms.fill(field.id, { kind: 'checked', checked: event.target.checked })}
        />
      )
    case 'radio':
      return (
        <input
          className={TOGGLE}
          style={style}
          aria-label={`${label} ${widget.onState ?? ''}`}
          type="radio"
          name={field.id}
          checked={widget.onState !== null && widget.onState === field.on}
          disabled={field.readOnly}
          onChange={() => void forms.fill(field.id, { kind: 'radio', widget: widget.id })}
        />
      )
    case 'combo':
    case 'list':
      return (
        <select
          className={TOGGLE}
          style={style}
          aria-label={label}
          multiple={field.kind === 'list'}
          value={field.kind === 'list' ? field.chosen : (field.chosen[0] ?? '')}
          disabled={field.readOnly}
          onChange={(event) =>
            void forms.fill(field.id, { kind: 'choices', choices: Array.from(event.target.selectedOptions, (option) => option.value) })
          }
        >
          {field.options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      )
    default:
      return null
  }
}

import type { CSSProperties, KeyboardEvent } from 'react'
import type { FormField, FormWidget } from '@/types/protocol.ts'
import { useFormsStore } from '@/stores/formsStore.ts'

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
          className="form-control form-text"
          style={style}
          aria-label={label}
          defaultValue={field.text}
          disabled={field.readOnly}
          onBlur={(event) => commit(event.target.value)}
        />
      ) : (
        <input
          key={field.text}
          className="form-control form-text"
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
          className="form-control form-toggle"
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
          className="form-control form-toggle"
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
          className="form-control form-choice"
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

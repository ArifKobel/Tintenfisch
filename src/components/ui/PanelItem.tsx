import { Icon, type IconName } from '@/components/ui/Icon.tsx'

export function PanelItem({ icon, label, chosen = false, onClick }: { icon: IconName; label: string; chosen?: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      className={`flex min-h-8 cursor-pointer items-center gap-3 rounded px-2 text-left ${chosen ? 'bg-selection-fill text-accent-ink' : 'text-ink-strong hover:bg-hover [&>svg]:text-ink'}`}
      aria-pressed={chosen}
      onClick={onClick}
    >
      <Icon name={icon} />
      {label}
    </button>
  )
}

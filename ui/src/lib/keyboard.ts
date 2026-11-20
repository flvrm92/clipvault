/** Keyboard map of the popup (specs/06 §2.2). Pure: decides an action from a key and context. */

export type KeyAction =
  | { type: 'move'; by: 1 | -1 }
  | { type: 'page'; direction: 1 | -1 }
  | { type: 'edge'; edge: 'first' | 'last' }
  | { type: 'paste'; mode: 'asIs' | 'trimmed' }
  | { type: 'pasteNth'; index: number }
  | { type: 'copy' }
  | { type: 'togglePin' }
  | { type: 'delete' }
  | { type: 'preview' }
  | { type: 'filter'; direction: 1 | -1 }
  | { type: 'clearSearch' }
  | { type: 'close' }
  | { type: 'settings' };

export interface KeyInput {
  key: string;
  shift: boolean;
  ctrl: boolean;
  alt: boolean;
}

/** State of the search box, which keeps keyboard focus while the popup is open. */
export interface KeyContext {
  searchEmpty: boolean;
  inputHasSelection: boolean;
  caretAtEnd: boolean;
}

/** The action for a key press, or `null` to let the search box handle it. */
export function resolveKey(input: KeyInput, context: KeyContext): KeyAction | null {
  const { key, shift, ctrl, alt } = input;

  if (alt && /^[1-9]$/.test(key)) return { type: 'pasteNth', index: Number(key) - 1 };
  if (ctrl && key.toLowerCase() === 'c') return context.inputHasSelection ? null : { type: 'copy' };
  if (ctrl && key.toLowerCase() === 'p') return { type: 'togglePin' };
  if (ctrl && key === ',') return { type: 'settings' };
  if (ctrl || alt) return null;

  switch (key) {
    case 'ArrowDown':
      return { type: 'move', by: 1 };
    case 'ArrowUp':
      return { type: 'move', by: -1 };
    case 'PageDown':
      return { type: 'page', direction: 1 };
    case 'PageUp':
      return { type: 'page', direction: -1 };
    case 'Home':
      return context.searchEmpty ? { type: 'edge', edge: 'first' } : null;
    case 'End':
      return context.searchEmpty ? { type: 'edge', edge: 'last' } : null;
    case 'Enter':
      return { type: 'paste', mode: shift ? 'trimmed' : 'asIs' };
    case 'Delete':
      return context.searchEmpty || context.caretAtEnd ? { type: 'delete' } : null;
    case ' ':
      return context.searchEmpty ? { type: 'preview' } : null;
    case 'Tab':
      return { type: 'filter', direction: shift ? -1 : 1 };
    case 'Escape':
      return context.searchEmpty ? { type: 'close' } : { type: 'clearSearch' };
    default:
      return null;
  }
}

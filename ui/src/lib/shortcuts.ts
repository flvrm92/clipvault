/** Shortcut text from a key press, for the shortcut recorder (HK-002). Uses physical key codes
 * so the result does not depend on the keyboard layout; the backend validates it (HK-003). */

const NAMED: Record<string, string> = {
  Space: 'Space',
  Enter: 'Enter',
  NumpadEnter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Insert: 'Insert',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Escape: 'Escape',
  PrintScreen: 'PrintScreen',
  Pause: 'Pause',
  Minus: 'Minus',
  Equal: 'Equal',
  Comma: 'Comma',
  Period: 'Period',
  Semicolon: 'Semicolon',
  Slash: 'Slash',
  Backquote: 'Backquote',
  BracketLeft: 'BracketLeft',
  BracketRight: 'BracketRight',
  Backslash: 'Backslash',
  Quote: 'Quote',
};

function keyName(code: string): string | null {
  const letter = /^Key([A-Z])$/.exec(code);
  if (letter) return letter[1] ?? null;
  const digit = /^Digit(\d)$/.exec(code);
  if (digit) return digit[1] ?? null;
  const numpad = /^Numpad(\d)$/.exec(code);
  if (numpad) return `Num${numpad[1]}`;
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;
  return NAMED[code] ?? null;
}

export interface KeyLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

/** `"Ctrl+Shift+1"`-style text, or `null` while only modifiers (or unknown keys) are pressed. */
export function comboFromEvent(event: KeyLike): string | null {
  const key = keyName(event.code);
  if (!key) return null;
  const parts = [
    event.ctrlKey && 'Ctrl',
    event.altKey && 'Alt',
    event.shiftKey && 'Shift',
    event.metaKey && 'Win',
    key,
  ].filter(Boolean);
  return parts.join('+');
}

/** Splits shortcut text into keycaps for display: `["Ctrl", "Win", "V"]`. */
export function keycaps(shortcut: string | null | undefined): string[] {
  return shortcut ? shortcut.split('+') : [];
}

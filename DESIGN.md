# Design Direction

## North Star

Quiet typing instrument: the interface should feel like a calm terminal surface with one task in front of the user.

## Visual World

- The terminal's native background shows through untouched, including terminal-level transparency or acrylic.
- Cool paper-colored text on whatever surface the user already has.
- A mint accent for the active choice, caret, and primary score.
- Muted blue-gray for untyped text and secondary metadata.
- Coral only for incorrect characters and recoverable storage errors.
- No dashboard card grid, rounded panels, decorative gradients, or rainbow status colors.

## Layout

- The typing text owns the center of the screen, shown as a three-line scrolling window with the active line in the middle (Monkeytype style), never the whole passage at once.
- One primary metric above the text: the countdown or word counter. Mode and progress are never shown twice.
- Settings are inline and compact on the menu.
- Live metrics are a quiet line below the text, not stat cards.
- Results and history use aligned rows and whitespace instead of outer panel borders.
- Preserve a deliberate 80x24 composition and degrade to a clear resize message below it.

## Typography

Use the terminal's monospace grid as the type system. Keep labels short and sentence case. Use deliberate spacing and aligned columns for changing numeric values so WPM and accuracy do not jump around.

## Interaction States

- Focus is shown with a warm accent marker and stronger text weight.
- Untyped letters are muted; correct letters become readable; incorrect letters are red and underlined where the terminal permits.
- The caret is a thin warm vertical marker.
- Actions are text rows with a single selection marker, not filled cards.

## Avoid

Generic admin-dashboard composition, boxed-in practice text, excessive borders, decorative copy, and visible shortcut tutorials inside the main surface.

#!/usr/bin/env python3
"""Generate the eraser cursor as raw RGBA pixel data.

Produces a hollow circle with 3 concentric strokes (white-black-white)
so that it remains visible against any background.

Requires: Pillow (pip install Pillow)

Usage: python scripts/gen_eraser_cursor.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

RADIUS = 10
SIZE = RADIUS * 2 + 1

img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)

# White outer ring
draw.ellipse([0, 0, SIZE - 1, SIZE - 1], outline=(255, 255, 255, 255), width=1)
# Black middle ring
draw.ellipse([1, 1, SIZE - 2, SIZE - 2], outline=(0, 0, 0, 255), width=1)
# White inner ring
draw.ellipse([2, 2, SIZE - 3, SIZE - 3], outline=(255, 255, 255, 255), width=1)

out = Path(__file__).resolve().parent.parent / "assets" / "eraser_cursor.rgba"
out.write_bytes(img.tobytes())
print(f"Wrote {out} ({SIZE}x{SIZE}, {out.stat().st_size} bytes)")

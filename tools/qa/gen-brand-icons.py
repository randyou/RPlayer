"""Rasterise the Sleeve R brand mark into the app icon set.

The mark is defined by `design/rplayer-v2/brand/rplayer-mark.svg`; there is no
SVG rasteriser available in this environment, so the geometry is rebuilt from
the same numbers the SVG uses:

    <rect x="2" y="2" width="60" height="60" rx="11" fill="#2E5B44"/>
    <path fill="#F4EFE5" d="M16 14h12v36H16z
                           M28 14h12c9.4 0 14 5.3 14 12s-4.6 12-14 12H28v-8h11
                             c3.2 0 5-1.5 5-4s-1.8-4-5-4H28z
                           M38.5 38 53 51H42L30.5 42z"/>

Run: python tools/qa/gen-brand-icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

FOREST = (0x2E, 0x5B, 0x44, 255)
PAPER = (0xF4, 0xEF, 0xE5, 255)
SUPERSAMPLE = 8
ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets"


def render(size: int) -> Image.Image:
    s = size * SUPERSAMPLE
    k = s / 64.0
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    def box(x0, y0, x1, y1):
        return [x0 * k, y0 * k, x1 * k, y1 * k]

    # Forest tile (rx 11 of 64).
    d.rounded_rectangle(box(2, 2, 62, 62), radius=11 * k, fill=FOREST)

    # Sleeve R: stem, bowl (square where it meets the stem, round on the right),
    # its counter, then the leg.
    d.rectangle(box(16, 14, 28, 50), fill=PAPER)
    # Pillow rejects a radius that exactly equals half the box height, so keep
    # one unit of headroom on the rounded corners.
    bowl_r = min(12 * k, (38 - 14) * k / 2 - 1)
    counter_r = min(4 * k, (30 - 22) * k / 2 - 1)
    d.rounded_rectangle(
        box(28, 14, 54, 38),
        radius=bowl_r,
        fill=PAPER,
        corners=[False, True, True, False],
    )
    d.rounded_rectangle(
        box(28, 22, 44, 30),
        radius=counter_r,
        fill=FOREST,
        corners=[False, True, True, False],
    )
    d.polygon(
        [
            (38.5 * k, 38 * k),
            (53 * k, 51 * k),
            (42 * k, 51 * k),
            (30.5 * k, 42 * k),
        ],
        fill=PAPER,
    )
    return img.resize((size, size), Image.LANCZOS)


def main():
    ASSETS.mkdir(exist_ok=True)
    frames = {}
    for size in (1024, 256, 48, 32, 24, 16):
        frames[size] = render(size)

    frames[1024].save(ASSETS / "icon.png")
    for size in (256, 48, 32, 24, 16):
        frames[size].save(ASSETS / f"icon-{size}.png")

    # Windows wants a multi-size .ico; Pillow writes every provided size.
    ico_sizes = [(256, 256), (48, 48), (32, 32), (24, 24), (16, 16)]
    frames[256].save(ASSETS / "icon.ico", format="ICO", sizes=ico_sizes)
    print("wrote", ", ".join(sorted(p.name for p in ASSETS.glob("icon*"))))


if __name__ == "__main__":
    main()

from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
MASTERS = ROOT / "masters"
APPICONSET = ROOT / "AppIcon.appiconset"
THUMBNAILSET = ROOT / "Thumbnail.imageset"
EXPORTS = ROOT / "exports"

APP_SOURCE = MASTERS / "timelet-app-icon-source.png"
THUMB_SOURCE = MASTERS / "timelet-thumbnail-source.png"

RESAMPLE = Image.Resampling.LANCZOS


def save_resized(image: Image.Image, path: Path, size: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    image.resize((size, size), RESAMPLE).save(path, optimize=True)


def make_app_masters() -> tuple[Image.Image, Image.Image]:
    source = Image.open(APP_SOURCE).convert("RGBA")

    # Crop away the generation margin while retaining the icon's soft corners.
    artwork = source.crop((80, 80, 1175, 1175)).resize((1024, 1024), RESAMPLE)

    macos = artwork

    # iOS marketing icons cannot contain alpha. Extend the sea palette behind the
    # artwork; the system-applied squircle mask hides the extreme corners.
    ios = Image.new("RGB", (1024, 1024))
    draw = ImageDraw.Draw(ios)
    top_left = (55, 158, 242)
    top_right = (42, 222, 235)
    bottom_left = (36, 172, 245)
    bottom_right = (45, 234, 220)
    for y in range(1024):
        fy = y / 1023
        left = tuple(round(top_left[i] * (1 - fy) + bottom_left[i] * fy) for i in range(3))
        right = tuple(round(top_right[i] * (1 - fy) + bottom_right[i] * fy) for i in range(3))
        for x in range(1024):
            fx = x / 1023
            color = tuple(round(left[i] * (1 - fx) + right[i] * fx) for i in range(3))
            draw.point((x, y), fill=color)
    ios.paste(artwork.convert("RGB"), mask=artwork.getchannel("A"))

    macos.save(MASTERS / "timelet-app-icon-macos-1024.png", optimize=True)
    ios.save(MASTERS / "timelet-app-icon-ios-1024.png", optimize=True)
    return macos, ios


def make_thumbnail_master() -> Image.Image:
    source = Image.open(THUMB_SOURCE).convert("RGB")
    gray = source.convert("L")

    # The generated preview contains a very pale checkerboard. Convert only the
    # dark glyph to alpha and discard the pale preview background.
    alpha = gray.point(lambda value: max(0, min(255, round((220 - value) * 255 / 140))))
    glyph = Image.new("RGBA", source.size, (0, 0, 0, 0))
    glyph.putalpha(alpha)

    bbox = alpha.getbbox()
    if bbox is None:
        raise RuntimeError("No black thumbnail glyph was detected")
    glyph = glyph.crop(bbox)
    side = max(glyph.size)
    padding = round(side * 0.12)
    canvas_side = side + padding * 2
    canvas = Image.new("RGBA", (canvas_side, canvas_side), (0, 0, 0, 0))
    canvas.alpha_composite(glyph, ((canvas_side - glyph.width) // 2, (canvas_side - glyph.height) // 2))
    canvas = canvas.resize((1024, 1024), RESAMPLE)
    canvas.save(MASTERS / "timelet-thumbnail-black-1024.png", optimize=True)
    return canvas


def export_app_icons(macos: Image.Image, ios: Image.Image) -> None:
    ios_sizes = {
        "timelet-ios-20.png": 20,
        "timelet-ios-20@2x.png": 40,
        "timelet-ios-20@3x.png": 60,
        "timelet-ios-29.png": 29,
        "timelet-ios-29@2x.png": 58,
        "timelet-ios-29@3x.png": 87,
        "timelet-ios-40.png": 40,
        "timelet-ios-40@2x.png": 80,
        "timelet-ios-40@3x.png": 120,
        "timelet-ios-60@2x.png": 120,
        "timelet-ios-60@3x.png": 180,
        "timelet-ios-76.png": 76,
        "timelet-ios-76@2x.png": 152,
        "timelet-ios-83.5@2x.png": 167,
        "timelet-ios-marketing-1024.png": 1024,
    }
    for filename, size in ios_sizes.items():
        save_resized(ios, APPICONSET / filename, size)

    mac_sizes = {
        "timelet-mac-16.png": 16,
        "timelet-mac-16@2x.png": 32,
        "timelet-mac-32.png": 32,
        "timelet-mac-32@2x.png": 64,
        "timelet-mac-128.png": 128,
        "timelet-mac-128@2x.png": 256,
        "timelet-mac-256.png": 256,
        "timelet-mac-256@2x.png": 512,
        "timelet-mac-512.png": 512,
        "timelet-mac-512@2x.png": 1024,
    }
    for filename, size in mac_sizes.items():
        save_resized(macos, APPICONSET / filename, size)

    for size in (16, 32, 64, 128, 256, 512, 1024):
        save_resized(ios, EXPORTS / f"timelet-app-icon-{size}.png", size)


def export_thumbnails(thumbnail: Image.Image) -> None:
    save_resized(thumbnail, THUMBNAILSET / "timelet-thumbnail.png", 32)
    save_resized(thumbnail, THUMBNAILSET / "timelet-thumbnail@2x.png", 64)
    save_resized(thumbnail, THUMBNAILSET / "timelet-thumbnail@3x.png", 96)
    for size in (16, 24, 32, 48, 64, 96, 128, 256, 512, 1024):
        save_resized(thumbnail, EXPORTS / f"timelet-thumbnail-black-{size}.png", size)


def main() -> None:
    APPICONSET.mkdir(parents=True, exist_ok=True)
    THUMBNAILSET.mkdir(parents=True, exist_ok=True)
    EXPORTS.mkdir(parents=True, exist_ok=True)
    macos, ios = make_app_masters()
    thumbnail = make_thumbnail_master()
    export_app_icons(macos, ios)
    export_thumbnails(thumbnail)


if __name__ == "__main__":
    main()

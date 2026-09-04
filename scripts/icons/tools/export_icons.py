import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
MASTERS = ROOT / "masters"
APPICONSET = ROOT / "AppIcon.appiconset"
THUMBNAILSET = ROOT / "Thumbnail.imageset"
EXPORTS = ROOT / "exports"
PROJECT_ROOT = ROOT.parents[1]
TAURI_ICONS = PROJECT_ROOT / "src-tauri" / "icons"
FRONTEND_ICON = PROJECT_ROOT / "src" / "assets" / "app-icon.png"
ICON_COMPOSER_SOURCE = TAURI_ICONS / "Timelet.icon"

APP_SOURCE = MASTERS / "timelet-app-icon-source.png"
MACOS_LEGACY_MASTER = MASTERS / "timelet-app-icon-macos-padded-1024.png"
THUMB_SOURCE = MASTERS / "timelet-thumbnail-source.png"

RESAMPLE = Image.Resampling.LANCZOS


def save_resized(image: Image.Image, path: Path, size: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    image.resize((size, size), RESAMPLE).save(path, optimize=True)


def make_app_masters() -> tuple[Image.Image, Image.Image, Image.Image]:
    source = Image.open(APP_SOURCE).convert("RGBA")

    # Crop away the generation margin while retaining the icon's soft corners.
    artwork = source.crop((80, 80, 1175, 1175)).resize((1024, 1024), RESAMPLE)

    transparent = artwork

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

    transparent.save(MASTERS / "timelet-app-icon-macos-1024.png", optimize=True)
    ios.save(MASTERS / "timelet-app-icon-ios-1024.png", optimize=True)

    # 传统 macOS 图标保留独立校准过的视觉边距。macOS 26 的系统蒙版
    # 使用无透明圆角的全幅母版，不能复用这里的双重留白版本。
    legacy_macos = Image.open(MACOS_LEGACY_MASTER).convert("RGBA")
    if legacy_macos.size != (1024, 1024):
        raise RuntimeError("Legacy macOS icon master must be 1024 x 1024")
    return transparent, ios, legacy_macos


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


def export_app_icons(legacy_macos: Image.Image, ios: Image.Image) -> None:
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
        save_resized(legacy_macos, APPICONSET / filename, size)

    for size in (16, 32, 64, 128, 256, 512, 1024):
        save_resized(ios, EXPORTS / f"timelet-app-icon-{size}.png", size)


def export_icns() -> None:
    pnpm = shutil.which("pnpm")
    if pnpm is None:
        raise RuntimeError("pnpm is required to export the macOS ICNS fallback")

    # 复用项目当前的桌面打包工具生成完整 ICNS 尺寸，避免不同系统版本的
    # iconutil 对同一 iconset 产生不一致结果。
    with tempfile.TemporaryDirectory(prefix="timelet-icon-") as temp_dir:
        output = Path(temp_dir) / "icons"
        subprocess.run(
            [
                pnpm,
                "exec",
                "tauri",
                "icon",
                str(MACOS_LEGACY_MASTER),
                "--output",
                str(output),
            ],
            cwd=PROJECT_ROOT,
            check=True,
        )
        shutil.copyfile(output / "icon.icns", TAURI_ICONS / "icon.icns")


def export_assets_car() -> None:
    if sys.platform != "darwin":
        print("Skipping Assets.car export outside macOS")
        return

    xcrun = shutil.which("xcrun")
    if xcrun is None:
        raise RuntimeError("xcrun is required to export the macOS 26 icon catalog")
    if not ICON_COMPOSER_SOURCE.exists():
        raise RuntimeError(f"Missing Icon Composer source: {ICON_COMPOSER_SOURCE}")

    # 当前系统工具无法单独编译 .icon 输入；同时提供空资源目录可保持输出
    # 内容不变，并规避系统工具接受独立图标源之前的编译崩溃。
    with tempfile.TemporaryDirectory(prefix="timelet-assets-") as temp_dir:
        temp = Path(temp_dir)
        catalog = temp / "TimeletAssets.xcassets"
        output = temp / "compiled"
        catalog.mkdir()
        output.mkdir()
        (catalog / "Contents.json").write_text(
            json.dumps({"info": {"author": "xcode", "version": 1}}),
            encoding="utf-8",
        )

        subprocess.run(
            [
                xcrun,
                "actool",
                str(ICON_COMPOSER_SOURCE),
                str(catalog),
                "--compile",
                str(output),
                "--output-format",
                "human-readable-text",
                "--notices",
                "--warnings",
                "--output-partial-info-plist",
                str(temp / "assetcatalog_generated_info.plist"),
                "--app-icon",
                "Timelet",
                "--include-all-app-icons",
                "--target-device",
                "mac",
                "--minimum-deployment-target",
                "26.0",
                "--platform",
                "macosx",
            ],
            check=True,
        )
        shutil.copyfile(output / "Assets.car", TAURI_ICONS / "Assets.car")


def export_runtime_icons(transparent: Image.Image, thumbnail: Image.Image) -> None:
    TAURI_ICONS.mkdir(parents=True, exist_ok=True)
    FRONTEND_ICON.parent.mkdir(parents=True, exist_ok=True)

    runtime_sizes = {
        "32x32.png": 32,
        "64x64.png": 64,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
    }
    for filename, size in runtime_sizes.items():
        save_resized(transparent, TAURI_ICONS / filename, size)

    windows_sizes = {
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "StoreLogo.png": 50,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
    }
    for filename, size in windows_sizes.items():
        save_resized(transparent, TAURI_ICONS / filename, size)

    transparent.save(
        TAURI_ICONS / "icon.ico",
        format="ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (256, 256)],
    )
    export_icns()
    save_resized(thumbnail, TAURI_ICONS / "tray.png", 44)
    save_resized(transparent, FRONTEND_ICON, 128)


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
    transparent, ios, legacy_macos = make_app_masters()
    thumbnail = make_thumbnail_master()
    export_app_icons(legacy_macos, ios)
    export_thumbnails(thumbnail)
    export_runtime_icons(transparent, thumbnail)
    export_assets_car()


if __name__ == "__main__":
    main()

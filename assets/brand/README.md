# forgecode brand assets

Source of truth: [`forgecode-icon.svg`](forgecode-icon.svg) (1024×1024, Terminal-Forge palette).

## Palette (Terminal-Forge, proposed 2026-07-06 by vision-pillar)

| Token | Hex | Role |
|---|---|---|
| deep-charcoal | `#0e0e10` | Background |
| deep-charcoal-2 | `#1c1c1f` | Window frame / panel secondary |
| amber-crt | `#f5a623` | Primary accent — CRT phosphor (the F> prompt) |
| synthwave-magenta | `#d946a8` | Secondary — AI glow / spark |
| mint-prompt | `#6ee7b7` | Tertiary — command-line success / echo line |

## Files

| Path | Format | Use |
|---|---|---|
| `assets/brand/forgecode-icon.svg` | SVG 1024×1024 | Source of truth |
| `assets/icons/forgecode.iconset/` | PNG 16/32/48/64/128/256/512/1024 + @2x | macOS `.icns` source |
| `assets/icons/forgecode.ico` | ICO multi-res 16/32/48/64/128/256 | Windows app icon |
| `assets/icons/forgecode-256x256.png` | PNG 256×256 | Linux app icon |

## Mark

A stylized terminal window with traffic-light dots (amber/magenta/mint), an amber-CRT `F>` prompt, a magenta spark glow (the AI hint), and a mint echo line below. Subtle CRT scanlines overlay the terminal body for the retro feel. Reads as "AI-enhanced terminal forge" — directly matches forgecode's brand position.

## Family position

- **Distinct from Tracera** (navy/teal/indigo, hex+diamond) — Terminal-Forge is monochrome charcoal + amber/magenta CRT.
- **Distinct from MelosViz** (warm orchestral palette, festival conductor) — Terminal-Forge is dark synthwave.
- **Distinct from Backbone-2** (sharecli/substrate infra family, graphite + green/violet) — Terminal-Forge uses amber/magenta, not green/violet. No hex overlap.
- **Distinct from Lab-Coat** (SessionLedger, light-mode white + cobalt) — Terminal-Forge is dark.

## Regeneration

```bash
# Re-export iconset from SVG (after editing forgecode-icon.svg)
for sz in 16 32 48 64 128 256 512 1024; do
  rsvg-convert -w $sz -h $sz assets/brand/forgecode-icon.svg \
    -o assets/icons/forgecode.iconset/icon_${sz}x${sz}.png
done
for sz in 16 32 128 256; do
  doubled=$((sz*2))
  cp assets/icons/forgecode.iconset/icon_${doubled}x${doubled}.png \
     assets/icons/forgecode.iconset/icon_${sz}x${sz}@2x.png
done

# Rebuild .ico (Windows)
convert assets/icons/forgecode.iconset/icon_{16,32,48,64,128,256}x{16,32,48,64,128,256}.png \
  assets/icons/forgecode.ico

# Linux 256
cp assets/icons/forgecode.iconset/icon_256x256.png assets/icons/forgecode-256x256.png
```

## Bundle wiring (forge_main Cargo.toml `[package.metadata.bundle]`)

The main `forge` binary lives in `crates/forge_main`. The bundle metadata
goes on that crate's manifest:

```toml
[package.metadata.bundle]
name = "forgecode"
identifier = "ai.kooshapari.forgecode"
icon = ["../../assets/icons/forgecode.iconset"]
resources = []
category = "DeveloperTool"
short_description = "AI-enhanced terminal development environment"
long_description = "Agentic coding CLI/TUI with ZSH plugin support."
```
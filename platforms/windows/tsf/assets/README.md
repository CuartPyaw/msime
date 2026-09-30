# TSF icon provenance

These thirteen unchanged ICO files are from `windows/image` in MSIME-Windows, remote default branch `develop`, fixed commit `04a8df56f86312474a069f4335a1b58da7afaa9e`.

They are the original assets referenced by that version's TSF resource script, not generated replacements. They retain the upstream repository's GPL-3.0 license; see the repository LICENSE. No neighboring working-tree files were copied. The DLL embeds the main, Chinese/English/Japanese/Korean/Caps Lock light/dark, character-width and punctuation mode icons through `IME/MetasequoiaIME.rc`.

`kr-light.ico` and `kr-dark.ico`, the Korean mode icons, are the two exceptions: upstream has no Korean mode, so they were drawn for this repository and carry its license. Each is the syllable 한 built from straight strokes and one ring on a 64-unit grid with 4-unit strokes, rasterized without any font at the same ten sizes as the Japanese icon (16 to 48 in steps of 4, and 64), black for light themes and white for dark ones, stored as 32-bit BMP entries like the files above.

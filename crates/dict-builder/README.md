# msime-dict-builder

`msime-dict-build` builds the dictionary release msime ships (the artifacts `resources/desktop-dictionary.lock.json` pins): `msime.db`, `english.db`, `others.db`, `dict_japanese.dat`, `bigram.bin`, `trigram.bin`, `mozc_dictionary_oss_README.txt`, then `dictionary-manifest.json` and `SHA256SUMS.txt`. It replaces the Python pipeline in MSIME-Engine `dictionary/` (`build_all.py`, `build_profile.py`, `makecikudb/`) and reproduces its desktop output row for row.

```sh
cargo build --release -p msime-dict-builder
target/release/msime-dict-build --cache <sources-cache> --out <output>                # every stage, about a minute
target/release/msime-dict-build --cache <sources-cache> --out <output> --skip ngram   # without the zhwiki pass
target/release/msime-dict-build --list
```

## Inputs

- `resources/dictionary-sources.lock.json` pins every third-party or large input by URL, size and SHA-256: the lexicons at a fixed msime-engine commit, ECDICT, Mozc's OSS dictionary and the zhwiki dump part the n-gram tables count. They are downloaded into `--cache` on first use (about 500 MB) and reused while they still match; `--offline` refuses to download.
- `resources/dictionary-sources/` holds the hand-maintained inputs: custom words and translations, quick phrases, the emoji, kaomoji and symbol tables, the single-character whitelist additions, and `pinyin-overrides.txt`.

Inputs without a redistribution grant (`src/licensing.rs`) are left out unless `--include-unlicensed` is given; such a build is for local evaluation and must not be released.

## Changing the data

- Edit the files under `resources/dictionary-sources/` and rebuild.
- Emoji, kaomoji and symbol keywords get their pinyin from the `pinyin` crate one character at a time. When a new polyphone keyword needs its phrase reading, add `keyword<TAB>item<TAB>item...` to `pinyin-overrides.txt`.
- To move a pinned input, change its URL, size and SHA-256 in the lock in the same commit, rebuild, and compare the result with the previous release before publishing.

After a release is published, bump `resources/desktop-dictionary.lock.json` to the new files.

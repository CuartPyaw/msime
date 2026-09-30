# msime-dict-builder

`msime-dict-build` builds the dictionary release msime ships (the artifacts `resources/desktop-dictionary.lock.json` pins): `msime.db`, `english.db`, `others.db`, `dict_japanese.dat`, `bigram.bin`, `trigram.bin`, `mozc_dictionary_oss_README.txt`, then `dictionary-manifest.json` and `SHA256SUMS.txt`. It replaces the Python pipeline in MSIME-Engine `dictionary/` (`build_all.py`, `build_profile.py`, `makecikudb/`) and reproduces its desktop output row for row.

```sh
cargo build --release -p msime-dict-builder
target/release/msime-dict-build --cache <sources-cache> --out <output>                # every stage, about a minute
target/release/msime-dict-build --cache <sources-cache> --out <output> --skip ngram   # without the zhwiki pass
target/release/msime-dict-build --list
```

## Inputs

- `resources/dictionary-sources.lock.json` pins every third-party or large input by URL, size and SHA-256: the lexicons at a fixed msime-engine commit, the shared custom dictionary, ECDICT, Mozc's OSS dictionary and the zhwiki dump part the n-gram tables count. They are downloaded into `--cache` on first use (about 500 MB) and reused while they still match; `--offline` refuses to download.
- The custom words and translations (`words.txt`, `translations.txt`) live in the shared custom dictionary [metasequoiaime/msime-customdict](https://github.com/metasequoiaime/msime-customdict), which msime and MSIME-Windows both consume through the dictionary release. The lock pins them at a fixed commit as `custom/words.txt` and `custom/translations.txt`.
- `resources/dictionary-sources/` holds the other hand-maintained inputs: quick phrases, the emoji, kaomoji and symbol tables, the single-character whitelist additions, and `pinyin-overrides.txt`.

Inputs without a redistribution grant (`src/licensing.rs`) are left out unless `--include-unlicensed` is given; such a build is for local evaluation and must not be released.

## Changing the data

- Edit the files under `resources/dictionary-sources/` and rebuild.
- Custom words and translations are changed in msime-customdict. To take its new state, move the `msime-customdict` reference and the two `custom/` entries in the lock to the new commit (URL, size and SHA-256), rebuild, and compare.
- Emoji, kaomoji and symbol keywords get their pinyin from the `pinyin` crate one character at a time. When a new polyphone keyword needs its phrase reading, add `keyword<TAB>item<TAB>item...` to `pinyin-overrides.txt`.
- To move a pinned input, change its URL, size and SHA-256 in the lock in the same commit, rebuild, and compare the result with the previous release before publishing.

After a release is published, bump `resources/desktop-dictionary.lock.json` to the new files.

## Checking contributed words

msime-customdict's CI gates changes to `words.txt` with `check-words`:

```sh
msime-dict-build check-words --base <old words.txt> --head <new words.txt> \
  [--msime-db <shipped msime.db>] [--json report.json] [--markdown summary.md]
```

A change may only append lines. Each appended entry goes through the parser the build uses (word, `'`-separated quanpin that maps to a table, integer weight), its weight must lie within the range the existing entries use, and it must not repeat another appended line, an entry already in `words.txt`, or (with `--msime-db`) a row with the same word and pinyin in the shipped quanpin tables. Appended blank and `#` lines are skipped. The JSON report lists `added` and `rejected` lines (1-based line number, text, reason) with `accepted`, the line counts and `weight_range`; the Markdown summary is the same in table form and is also printed to stderr. The exit status is 0 when nothing is rejected, 1 when anything is (the reports are still written), and 2 when the check cannot run: an unreadable file, a base `words.txt` the build itself would reject, or a database without the expected table.

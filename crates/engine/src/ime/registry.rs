//! The providers, one per scheme family (`R/providers/provider_registry.cpp`, `pinyin_candidate_provider.cpp`): pinyin (quanpin and shuangpin engines), wubi and Japanese.

use crate::assets;
use crate::error::Result;
use crate::helpcode::SharedKeymap;
use crate::japanese::JapaneseProvider;
use crate::paths::RuntimePaths;
use crate::quanpin::QuanpinEngine;
use crate::shuangpin::profile::profile;
use crate::shuangpin::ShuangpinEngine;
use crate::types::{CandidateSource, QueryRequest, SchemeType, ShuangpinProfileKind, WordItem};
use crate::wubi::provider::WubiProvider;

pub struct ProviderRegistry {
    pub quanpin: QuanpinEngine,
    pub shuangpin: ShuangpinEngine,
    pub wubi: WubiProvider,
    pub japanese: JapaneseProvider,
    keymap: Option<SharedKeymap>,
}

impl ProviderRegistry {
    /// Wubi reads the generation's `msime.db` and journals into the user journal; the Japanese model is the immutable resource (provider_registry.cpp:4-10).
    pub fn new(profile_kind: ShuangpinProfileKind, paths: &RuntimePaths) -> Self {
        Self {
            quanpin: QuanpinEngine::new(paths),
            shuangpin: ShuangpinEngine::new(profile(profile_kind), paths),
            wubi: WubiProvider::new(
                &paths.dictionary(assets::MAIN_DICTIONARY),
                &paths.user(assets::USER_JOURNAL),
            ),
            japanese: JapaneseProvider::new(&paths.resource(assets::JAPANESE_MODEL)),
            keymap: None,
        }
    }

    pub fn set_helpcode_keymap(&mut self, keymap: Option<SharedKeymap>) {
        self.keymap = keymap;
    }

    /// Pinyin rows are stamped with the request's scheme (pinyin_candidate_provider.cpp:12-28); wubi rows carry `Wubi` from their provider, and Japanese rows keep the default scheme, as the reference recorded them.
    pub fn query(&mut self, request: &QueryRequest) -> Vec<WordItem> {
        if !request.valid {
            return Vec::new();
        }
        let keymap = self.keymap.as_deref();
        let mut candidates = match request.scheme {
            SchemeType::Quanpin => self.quanpin.query(request, keymap),
            SchemeType::Shuangpin => self.shuangpin.query(request, keymap),
            SchemeType::Wubi => return self.wubi.query(request),
            SchemeType::JapaneseRomaji => return self.japanese.query(request),
        };
        for item in &mut candidates {
            item.scheme = request.scheme;
        }
        candidates
    }

    /// Wubi and Japanese never answer a lookup (wubi_candidate_provider.h:19-22; the Japanese one read the dropped `japanese_lexicon`).
    pub fn find_candidate(&self, scheme: SchemeType, key: &str, value: &str) -> Option<WordItem> {
        match scheme {
            SchemeType::Quanpin => self.quanpin.find_candidate(key, value),
            SchemeType::Shuangpin => self.shuangpin.find_candidate(key, value),
            SchemeType::Wubi | SchemeType::JapaneseRomaji => None,
        }
    }

    /// Either pinyin scheme resets both pinyin engines (pinyin_candidate_provider.cpp:44-48).
    pub fn reset_cache(&mut self, scheme: SchemeType) {
        match scheme {
            SchemeType::Quanpin | SchemeType::Shuangpin => {
                self.quanpin.reset_cache();
                self.shuangpin.reset_cache();
            }
            SchemeType::Wubi => self.wubi.reset_cache(),
            SchemeType::JapaneseRomaji => self.japanese.reset_cache(),
        }
    }

    /// Wubi and Japanese store no user words (`kNoMutation`).
    pub fn create_word(&mut self, scheme: SchemeType, pinyin: &str, word: &str) -> Result<()> {
        match scheme {
            SchemeType::Quanpin => self.quanpin.create_word(pinyin, word),
            SchemeType::Shuangpin => self.shuangpin.create_word(pinyin, word),
            SchemeType::Wubi | SchemeType::JapaneseRomaji => Ok(()),
        }
    }

    pub fn update_weight_by_pinyin_and_word(
        &mut self,
        scheme: SchemeType,
        pinyin: &str,
        word: &str,
    ) -> Result<()> {
        match scheme {
            SchemeType::Quanpin => self.quanpin.update_weight_by_pinyin_and_word(pinyin, word),
            SchemeType::Shuangpin => self
                .shuangpin
                .update_weight_by_pinyin_and_word(pinyin, word),
            SchemeType::Wubi => self.wubi.update_weight(pinyin, word),
            SchemeType::JapaneseRomaji => Ok(()),
        }
    }

    pub fn delete_by_pinyin_and_word(
        &mut self,
        scheme: SchemeType,
        pinyin: &str,
        word: &str,
    ) -> Result<()> {
        match scheme {
            SchemeType::Quanpin => self.quanpin.delete_by_pinyin_and_word(pinyin, word),
            SchemeType::Shuangpin => self.shuangpin.delete_by_pinyin_and_word(pinyin, word),
            SchemeType::Wubi => self.wubi.delete(pinyin, word),
            SchemeType::JapaneseRomaji => Ok(()),
        }
    }

    /// Online rows for a request: quanpin and shuangpin series caches, the Japanese dynamic row. Wubi and Japanese take a batch only through the single-word default of `ICandidateProvider` (candidate_provider.h:27-31); wubi then accepts and drops it (wubi_candidate_provider.cpp:102-106).
    pub fn cache_dynamic_candidates_for_request(
        &mut self,
        request: &QueryRequest,
        words: &[String],
        source: CandidateSource,
    ) -> bool {
        match request.scheme {
            SchemeType::Quanpin => self.quanpin.insert_online_words(request, words, source),
            SchemeType::Shuangpin => self.shuangpin.insert_online_words(request, words, source),
            SchemeType::Wubi => words.len() == 1,
            SchemeType::JapaneseRomaji => match words {
                [word] => self
                    .japanese
                    .cache_dynamic_candidate(&request.raw_input, word, source),
                _ => false,
            },
        }
    }

    /// Always the pinyin provider, which answers only quanpin and shuangpin requests (pinyin_candidate_provider.cpp:31-42).
    pub fn expand_initial_candidates(
        &mut self,
        request: &QueryRequest,
        candidates: &mut Vec<WordItem>,
    ) -> bool {
        match request.scheme {
            SchemeType::Quanpin => self.quanpin.expand_initial_candidates(request, candidates),
            SchemeType::Shuangpin => self
                .shuangpin
                .expand_initial_candidates(request, candidates),
            SchemeType::Wubi | SchemeType::JapaneseRomaji => false,
        }
    }
}

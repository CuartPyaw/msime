//! Neural reranking of the lattice's n-best (overlays.md §1.6.2-§1.6.3) on the `chinese-ime-lm` crate. The model never generates sentences; it only reorders lattice paths, and each enabled source contributes one row. Scoring is synchronous on the session's own `Reranker`, whose prefix cache keeps a keystroke cheap.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use chinese_ime_lm::{Reranker, SentenceModel};

use super::decode::SentencePath;
use crate::text::last_characters;
use crate::types::CandidateSource;

pub const RERANK_LAMBDA: f64 = 0.5;
pub const MAX_RERANK_PATHS: usize = 12;
pub const CONTEXT_CHARACTERS: usize = 64;
/// The shipped desktop model is about 25 MiB; this leaves room for a larger compatible model without letting a configured path make startup allocate without bound.
pub const MAX_MODEL_BYTES: u64 = 64 * 1024 * 1024;

/// The lattice score is converted as log10 the way the C++ did (patch:869-871); the arithmetic is ported unchanged so the two models' blend keeps the weight it was tuned with.
const LOG10_TO_NATS: f64 = std::f64::consts::LN_10;

/// One model per path for the process; a load failure is remembered as `None`, so a missing file is not re-read on every keystroke (patch:902-912).
pub fn shared_sentence_model(path: &Path) -> Option<Arc<SentenceModel>> {
    static MODELS: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<SentenceModel>>>>> = OnceLock::new();
    let mut models = MODELS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    models
        .entry(path.to_path_buf())
        .or_insert_with(|| load_model(path))
        .clone()
}

/// A missing, oversized or malformed model means no neural rows, never a failed session: the lattice still answers.
fn load_model(path: &Path) -> Option<Arc<SentenceModel>> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_MODEL_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    // Bounded again while reading, in case the file grew after the size check.
    file.take(MAX_MODEL_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_MODEL_BYTES {
        return None;
    }
    SentenceModel::load(&bytes).ok().map(Arc::new)
}

/// The first `min(n, 12)` indices stable-sorted by `static * ln 10 + lambda * (neural - static * ln 10)` descending, the rest in order. `None` for fewer than two paths or mismatched lengths (patch:914-944). `neural` holds per-sentence summed natural-log probabilities, `static_log10` the lattice scores.
///
/// `neural` scores the same paths as `static_log10`, in order; only its first `min(n, 12)` entries are read, so a caller may score only those and pass exactly that many.
pub fn rerank_order(neural: &[f64], static_log10: &[f64]) -> Option<Vec<usize>> {
    let n = static_log10.len();
    let scored = n.min(MAX_RERANK_PATHS);
    if n < 2 || (neural.len() != n && neural.len() != scored) {
        return None;
    }
    let combined: Vec<f64> = (0..scored)
        .map(|index| {
            let statik = static_log10[index] * LOG10_TO_NATS;
            statik + RERANK_LAMBDA * (neural[index] - statik)
        })
        .collect();
    let mut order: Vec<usize> = (0..n).collect();
    // Stable so that sentences the two models score identically keep the lattice's order, and the unscored tail past the cap stays as it was.
    order[..scored].sort_by(|&a, &b| combined[b].total_cmp(&combined[a]));
    Some(order)
}

pub struct NeuralReranker {
    pub source: CandidateSource,
    reranker: Reranker,
}

impl NeuralReranker {
    /// `source` is `NeuralKeyboard` or `NeuralDesktop`.
    pub fn new(source: CandidateSource, model: Arc<SentenceModel>) -> Self {
        Self {
            source,
            reranker: Reranker::new(model),
        }
    }

    /// Reorder `paths` by `rerank_order` conditioned on the last 64 characters of `context`; false leaves them untouched. The crate returns mean per-character log-probabilities; the sum is that times the character count.
    pub fn rerank(&mut self, paths: &mut [SentencePath], context: &str) -> bool {
        if paths.len() < 2 {
            return false;
        }
        let scored = paths.len().min(MAX_RERANK_PATHS);
        let texts: Vec<&str> = paths[..scored]
            .iter()
            .map(|path| path.sentence.as_str())
            .collect();
        let means = self
            .reranker
            .log_probabilities(last_characters(context, CONTEXT_CHARACTERS), &texts);
        // A model that answered for some sentences only is inconsistent; half a reranking is worse than none, so the lattice order stays.
        if means.len() != scored {
            return false;
        }
        let neural: Vec<f64> = means
            .iter()
            .zip(&texts)
            .map(|(mean, text)| f64::from(*mean) * text.chars().count() as f64)
            .collect();
        let statik: Vec<f64> = paths.iter().map(|path| path.log_prob).collect();
        let Some(order) = rerank_order(&neural, &statik) else {
            return false;
        };
        let reordered: Vec<SentencePath> =
            order.iter().map(|&index| paths[index].clone()).collect();
        paths.clone_from_slice(&reordered);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::decode::tests::{lookup, syllables, table};
    use super::super::decode::LatticeOptions;
    use super::super::merge::merge_lattice_candidates;
    use super::*;
    use crate::assets;
    use crate::types::WordItem;

    fn path(sentence: &str, log_prob: f64) -> SentencePath {
        SentencePath {
            sentence: sentence.into(),
            key: String::new(),
            log_prob,
            words: vec![sentence.into()],
            typo_edges: 0,
        }
    }

    #[test]
    fn rerank_order_blends_the_two_scores() {
        // Static in log10, neural in nats: path 1 is worse on the lattice but the model prefers it enough.
        let statik = [-1.0, -1.2];
        let neural = [-10.0, -2.0];
        let combined0 = -LOG10_TO_NATS + 0.5 * (-10.0 + LOG10_TO_NATS);
        let combined1 = -1.2 * LOG10_TO_NATS + 0.5 * (-2.0 + 1.2 * LOG10_TO_NATS);
        assert!(combined1 > combined0);
        assert_eq!(rerank_order(&neural, &statik), Some(vec![1, 0]));
        // The model agreeing with the lattice leaves the order.
        assert_eq!(rerank_order(&[-2.0, -10.0], &statik), Some(vec![0, 1]));
    }

    #[test]
    fn rerank_order_is_stable_on_ties() {
        assert_eq!(
            rerank_order(&[-1.0, -1.0, -1.0], &[-1.0, -1.0, -1.0]),
            Some(vec![0, 1, 2])
        );
    }

    #[test]
    fn rerank_order_declines_what_it_cannot_order() {
        assert_eq!(rerank_order(&[], &[]), None);
        assert_eq!(
            rerank_order(&[-1.0], &[-1.0]),
            None,
            "one path has nothing to reorder"
        );
        assert_eq!(rerank_order(&[-1.0], &[-1.0, -2.0]), None);
        assert_eq!(rerank_order(&[-1.0, -2.0, -3.0], &[-1.0, -2.0]), None);
    }

    #[test]
    fn rerank_order_leaves_the_tail_past_twelve() {
        let n = 15;
        let statik: Vec<f64> = (0..n).map(|index| -(index as f64)).collect();
        // The model reverses the scored twelve and would reverse the tail too if it were read.
        let neural: Vec<f64> = (0..n).map(|index| index as f64 * 100.0).collect();
        let order = rerank_order(&neural, &statik).unwrap();
        assert_eq!(&order[..12], &(0..12).rev().collect::<Vec<_>>()[..]);
        assert_eq!(&order[12..], &[12, 13, 14]);
        // Scoring only the first twelve is the same answer.
        assert_eq!(rerank_order(&neural[..12], &statik), Some(order));
    }

    #[test]
    fn rerank_order_constants() {
        assert_eq!(RERANK_LAMBDA, 0.5);
        assert_eq!(MAX_RERANK_PATHS, 12);
        assert_eq!(CONTEXT_CHARACTERS, 64);
        assert_eq!(last_characters("a你好", 2), "你好");
        assert_eq!(last_characters("a你好", 0), "");
    }

    #[test]
    fn a_missing_model_is_remembered() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(assets::NEURAL_MODEL_KEYBOARD);
        assert!(shared_sentence_model(&path).is_none());
        std::fs::write(&path, b"not a model").unwrap();
        assert!(shared_sentence_model(&path).is_none());
        let garbage = directory.path().join("garbage.safetensors");
        std::fs::write(&garbage, b"not a model").unwrap();
        assert!(
            shared_sentence_model(&garbage).is_none(),
            "a malformed model loads as nothing"
        );
    }

    /// The keyboard model from `MSIME_EVAL_RESOURCES`, or the reason the test cannot run.
    fn resource_model(name: &str) -> Result<Arc<SentenceModel>, String> {
        let resources = std::env::var_os("MSIME_EVAL_RESOURCES")
            .ok_or_else(|| "MSIME_EVAL_RESOURCES is not set".to_owned())?;
        let path = PathBuf::from(resources).join(name);
        if !path.is_file() {
            return Err(format!("{} is not in MSIME_EVAL_RESOURCES", name));
        }
        shared_sentence_model(&path).ok_or_else(|| format!("{} did not load", path.display()))
    }

    #[test]
    fn a_real_model_scores_and_reorders() {
        let model = match resource_model(assets::NEURAL_MODEL_KEYBOARD) {
            Ok(model) => model,
            Err(reason) => {
                eprintln!("skipping a_real_model_scores_and_reorders: {reason}");
                return;
            }
        };
        let mut reranker = NeuralReranker::new(CandidateSource::NeuralKeyboard, model.clone());
        let mut paths = vec![path("输入发", -5.0), path("输入法", -5.1)];
        assert!(reranker.rerank(&mut paths, "我在用一个新的"));
        assert_eq!(
            paths[0].sentence, "输入法",
            "the model knows the common word"
        );
        let mut single = vec![path("输入法", -5.0)];
        assert!(!reranker.rerank(&mut single, ""));

        // The sum the reranker uses is the crate's mean times the length.
        let means = Reranker::new(model).log_probabilities("", &["输入法"]);
        assert_eq!(means.len(), 1);
        assert!(means[0] < 0.0);
    }

    #[test]
    fn two_rerankers_emit_one_path_each() {
        let (keyboard, desktop) = match (
            resource_model(assets::NEURAL_MODEL_KEYBOARD),
            resource_model(assets::NEURAL_MODEL_DESKTOP),
        ) {
            (Ok(keyboard), Ok(desktop)) => (keyboard, desktop),
            (Err(reason), _) | (_, Err(reason)) => {
                eprintln!("skipping two_rerankers_emit_one_path_each: {reason}");
                return;
            }
        };
        let rows = table(&[
            ("shu'ru", &[("输入", 20000)]),
            ("fa", &[("法", 800000), ("发", 900000), ("罚", 100000)]),
        ]);
        let mut rerankers = vec![
            NeuralReranker::new(CandidateSource::NeuralKeyboard, keyboard),
            NeuralReranker::new(CandidateSource::NeuralDesktop, desktop),
        ];
        let options = LatticeOptions {
            nbest: MAX_RERANK_PATHS,
            show_next_on_duplicate: true,
            ..LatticeOptions::default()
        };
        let mut candidates: Vec<WordItem> = Vec::new();
        merge_lattice_candidates(
            &mut candidates,
            &syllables("shu'ru'fa"),
            &mut lookup(&rows),
            "shurufa",
            &options,
            None,
            &mut rerankers,
            "",
        );
        assert_eq!(candidates[0].source, CandidateSource::Generated);
        assert_eq!(
            candidates[0].word, "输入发",
            "the lattice best is unreranked"
        );
        let sources: Vec<_> = candidates.iter().map(|item| item.source).collect();
        assert_eq!(
            sources
                .iter()
                .filter(|source| **source == CandidateSource::NeuralKeyboard)
                .count(),
            1
        );
        assert_eq!(
            sources
                .iter()
                .filter(|source| **source == CandidateSource::NeuralDesktop)
                .count(),
            1
        );
        assert_eq!(candidates.len(), 3);
        let words: std::collections::HashSet<_> =
            candidates.iter().map(|item| item.word.as_str()).collect();
        assert_eq!(words.len(), 3, "each source adds a distinct sentence");
    }
}

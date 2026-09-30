//! Personal context reorder of the leading homophone group (core-session.md §7.2, `R/core/personal_context_rerank.cpp`).

use crate::lattice::personal::PersonalNgram;
use crate::types::WordItem;

pub const MAX_GROUP: usize = 16;

/// The reordered list when the model moves something in the leading same-length dictionary group, else `None`. A pinned leader (`is_pinned(word)`) keeps its seat.
pub fn personal_context_rerank(
    candidates: &[WordItem],
    model: &PersonalNgram,
    earlier: Option<&str>,
    previous: &str,
    is_pinned: &mut dyn FnMut(&str) -> bool,
) -> Option<Vec<WordItem>> {
    let first = candidates.first()?;
    if candidates.len() < 2 || !first.source.is_dictionary() {
        return None;
    }
    let context = model.context(earlier, Some(previous));
    let mu = context.confidence;
    if mu <= 0.0 {
        return None;
    }

    let group = leading_group_len(candidates);
    if group < 2 {
        return None;
    }
    let personal: Vec<f64> = candidates[..group]
        .iter()
        .map(|item| model.probability(&context, &item.word))
        .collect();
    let order = blended_order(&candidates[..group], &personal, mu, is_pinned)?;

    let mut ordered = Vec::with_capacity(candidates.len());
    ordered.extend(order.into_iter().map(|index| candidates[index].clone()));
    ordered.extend_from_slice(&candidates[group..]);
    Some(ordered)
}

/// The leading run, at most 16 rows, of dictionary rows with as many characters as the first (code points, like `count_utf8_chars`).
fn leading_group_len(candidates: &[WordItem]) -> usize {
    let characters = candidates[0].word.chars().count();
    candidates
        .iter()
        .take(MAX_GROUP)
        .take_while(|item| item.source.is_dictionary() && item.word.chars().count() == characters)
        .count()
}

/// Indices of `group` ordered by `ln((1 - mu) * share + mu * personal / sum(personal))`, where `share` is the row's weight (at least 1) over the group's; `None` when the model knows none of the words or the order would not change.
fn blended_order(
    group: &[WordItem],
    personal: &[f64],
    mu: f64,
    is_pinned: &mut dyn FnMut(&str) -> bool,
) -> Option<Vec<usize>> {
    let personal_sum: f64 = personal.iter().sum();
    if personal_sum <= 0.0 {
        return None;
    }
    let weight = |item: &WordItem| item.weight.max(1) as f64;
    let weight_sum: f64 = group.iter().map(weight).sum();

    let mut scored: Vec<(f64, usize)> = group
        .iter()
        .zip(personal)
        .enumerate()
        .map(|(index, (item, probability))| {
            let share = weight(item) / weight_sum;
            (
                ((1.0 - mu) * share + mu * probability / personal_sum).ln(),
                index,
            )
        })
        .collect();
    // Stable, so equal scores keep the dictionary order.
    scored.sort_by(|left, right| right.0.total_cmp(&left.0));

    // The pin lookup reads the journal, so it is only asked when the leader would actually lose its seat.
    if scored[0].1 != 0 && is_pinned(&group[0].word) {
        let leader = scored
            .iter()
            .position(|&(_, index)| index == 0)
            .expect("the leader is scored");
        scored[..=leader].rotate_right(1);
    }

    let order: Vec<usize> = scored.into_iter().map(|(_, index)| index).collect();
    if order.iter().enumerate().all(|(seat, &index)| seat == index) {
        return None;
    }
    Some(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CandidateSource;

    fn row(word: &str, weight: i64) -> WordItem {
        WordItem::new("ni", word, weight, CandidateSource::Database, "ni")
    }

    fn never_pinned() -> impl FnMut(&str) -> bool {
        |_: &str| false
    }

    #[test]
    fn a_strong_context_moves_the_follower_up() {
        let group = [row("甲", 300), row("乙", 200), row("丙", 100)];
        let order = blended_order(&group, &[0.0, 0.0, 1.0], 0.5, &mut never_pinned());
        assert_eq!(order, Some(vec![2, 0, 1]));
    }

    #[test]
    fn a_weak_context_keeps_the_dictionary_order() {
        // A context word the dictionary already ranks second cannot unseat a far heavier leader.
        let group = [row("甲", 1_000_000), row("乙", 10), row("丙", 1)];
        assert_eq!(
            blended_order(&group, &[0.0, 1.0, 0.0], 0.01, &mut never_pinned()),
            None
        );
        // The same weak context still lifts an unweighted follower past a row it knows nothing about.
        assert_eq!(
            blended_order(&group, &[0.0, 0.0, 1.0], 0.01, &mut never_pinned()),
            Some(vec![0, 2, 1])
        );
    }

    #[test]
    fn an_unknown_context_changes_nothing() {
        let group = [row("甲", 3), row("乙", 2)];
        assert_eq!(
            blended_order(&group, &[0.0, 0.0], 0.5, &mut never_pinned()),
            None
        );
    }

    #[test]
    fn weights_below_one_count_as_one() {
        // With equal shares the personal term alone decides.
        let group = [row("甲", 0), row("乙", -5)];
        assert_eq!(
            blended_order(&group, &[0.2, 0.8], 0.5, &mut never_pinned()),
            Some(vec![1, 0])
        );
    }

    #[test]
    fn ties_keep_the_dictionary_order() {
        let group = [row("甲", 1), row("乙", 1), row("丙", 1)];
        assert_eq!(
            blended_order(&group, &[0.0, 0.5, 0.5], 0.5, &mut never_pinned()),
            Some(vec![1, 2, 0])
        );
    }

    #[test]
    fn a_pinned_leader_keeps_its_seat_and_is_asked_only_when_it_would_move() {
        let group = [row("甲", 300), row("乙", 200), row("丙", 100)];
        let mut asked = Vec::new();
        let order = blended_order(&group, &[0.0, 0.3, 0.7], 0.9, &mut |word: &str| {
            asked.push(word.to_string());
            true
        });
        assert_eq!(order, Some(vec![0, 2, 1]));
        assert_eq!(asked, vec!["甲".to_string()]);

        let mut asked = 0;
        let order = blended_order(&group, &[0.9, 0.0, 0.1], 0.9, &mut |_: &str| {
            asked += 1;
            true
        });
        assert_eq!(order, Some(vec![0, 2, 1]));
        assert_eq!(asked, 0);
    }

    #[test]
    fn the_group_is_the_leading_same_length_dictionary_run() {
        let mut list = vec![row("甲", 3), row("乙", 2), row("丙丁", 1), row("戊", 1)];
        assert_eq!(leading_group_len(&list), 2);
        list[1].source = CandidateSource::CloudSuggestion;
        assert_eq!(leading_group_len(&list), 1);
        list[1].source = CandidateSource::UserDatabase;
        list[2] = row("丙", 1);
        assert_eq!(leading_group_len(&list), 4);

        let long: Vec<WordItem> = (0..20).map(|index| row("字", index)).collect();
        assert_eq!(leading_group_len(&long), MAX_GROUP);
    }
}

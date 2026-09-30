//! `V` mode: an arithmetic expression evaluated by `exmex`, a number written out in Chinese numerals by `chinese-number`, or a `YYYY.M.D` date. Rows are computed on the key that changed the input and read nothing from disk; an input that is none of these (an operator still waiting for its operand) has no rows, so the session shows the raw text instead.

use chinese_number::{ChineseCase, ChineseCountMethod, ChineseVariant, NumberToChinese};
use exmex::{ops_factory, BinOp, Express, FlatEx, MakeOperators, Operator};
use time::{Date, Month};

use super::date_time::{chinese_number, lunar_date, year_digits, LocalDateTime, WEEKDAYS};
use crate::types::{CandidateSource, WordItem};

/// Digits and the arithmetic the mode spells with; letters are not part of it.
pub const SPELLING_SYMBOLS: &str = "0123456789+-*/.()%^";
/// No input produces more rows than one page of the nine-row Windows candidate window.
pub const RESULT_LIMIT: usize = 9;
/// Numbers from here on are not written out in Chinese numerals: past 万亿 the readings stop being something people write.
const CHINESE_NUMERAL_LIMIT: u64 = 1_000_000_000_000_000;
/// Results are shown to this many significant digits, which hides binary rounding: 0.1+0.2 reads 0.3.
const SIGNIFICANT_DIGITS: usize = 12;

// The default float operators minus the named functions and constants, which the mode cannot spell, plus `%` as the remainder. The priorities are exmex's own: `/` and `%` bind tighter than `*`, `-` tighter than `+`.
ops_factory!(
    ArithmeticOps,
    f64,
    Operator::make_bin(
        "^",
        BinOp {
            apply: f64::powf,
            prio: 4,
            is_commutative: false,
        }
    ),
    Operator::make_bin(
        "*",
        BinOp {
            apply: |a, b| a * b,
            prio: 2,
            is_commutative: true,
        }
    ),
    Operator::make_bin(
        "/",
        BinOp {
            apply: |a, b| a / b,
            prio: 3,
            is_commutative: false,
        }
    ),
    Operator::make_bin(
        "%",
        BinOp {
            apply: |a, b| a % b,
            prio: 3,
            is_commutative: false,
        }
    ),
    Operator::make_bin_unary(
        "+",
        BinOp {
            apply: |a, b| a + b,
            prio: 0,
            is_commutative: true,
        },
        |a| a
    ),
    Operator::make_bin_unary(
        "-",
        BinOp {
            apply: |a, b| a - b,
            prio: 1,
            is_commutative: false,
        },
        |a| -a
    )
);

/// Generated rows for the input after `V`, weight `count - index`, at most `RESULT_LIMIT`.
pub fn query_expression(code: &str) -> Vec<WordItem> {
    if code.is_empty()
        || !code
            .chars()
            .all(|character| SPELLING_SYMBOLS.contains(character))
    {
        return Vec::new();
    }
    let texts = if let Some(date) = parse_date(code) {
        date_rows(date)
    } else if code
        .bytes()
        .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        number_rows(code)
    } else {
        expression_rows(code)
    };
    let count = texts.len().min(RESULT_LIMIT);
    texts
        .into_iter()
        .take(count)
        .enumerate()
        .map(|(index, text)| {
            WordItem::new(
                "",
                text,
                (count - index) as i64,
                CandidateSource::Generated,
                "",
            )
        })
        .collect()
}

/// The result, the equation, and the amount in capitals when the result is one.
fn expression_rows(code: &str) -> Vec<String> {
    let Some(value) = evaluate(code) else {
        return Vec::new();
    };
    let Some(result) = format_number(value) else {
        return Vec::new();
    };
    let mut rows = vec![result.clone(), format!("{code}={result}")];
    rows.extend(amount_in_capitals(&result));
    rows
}

fn evaluate(code: &str) -> Option<f64> {
    let expression = FlatEx::<f64, ArithmeticOps>::parse(code).ok()?;
    // The mode cannot spell a name, but a variable would make `eval` fail rather than answer anyway.
    if !expression.var_names().is_empty() {
        return None;
    }
    expression
        .eval(&[])
        .ok()
        .filter(|value: &f64| value.is_finite())
}

/// `SIGNIFICANT_DIGITS` significant digits, positional between one millionth and 10^16 and in exponent form outside it; `None` for infinity and NaN.
fn format_number(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let rounded: f64 = format!("{value:.precision$e}", precision = SIGNIFICANT_DIGITS - 1)
        .parse()
        .ok()?;
    // Adding zero turns -0 into 0, so 1-1 and -0 both read 0.
    let rounded = rounded + 0.0;
    let magnitude = rounded.abs();
    Some(if magnitude != 0.0 && !(1e-6..1e16).contains(&magnitude) {
        format!("{rounded:e}")
    } else {
        rounded.to_string()
    })
}

/// A plain number: lowercase, capitals and the amount for an integer; the reading with 点 and, for at most two decimals, the amount for a decimal.
fn number_rows(code: &str) -> Vec<String> {
    let (integer, fraction) = match code.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (code, None),
    };
    if integer.is_empty()
        || fraction.is_some_and(|fraction| fraction.is_empty() || fraction.contains('.'))
    {
        return Vec::new();
    }
    let Some(value) = integer
        .parse::<u64>()
        .ok()
        .filter(|value| *value < CHINESE_NUMERAL_LIMIT)
    else {
        return Vec::new();
    };
    let Some(lower) = chinese_integer(value, ChineseCase::Lower) else {
        return Vec::new();
    };
    let Some(fraction) = fraction else {
        let mut rows = vec![lower];
        rows.extend(chinese_integer(value, ChineseCase::Upper));
        rows.extend(amount_in_capitals(code));
        return rows;
    };
    let digits: Option<String> = fraction
        .bytes()
        .map(|digit| chinese_digit(digit - b'0', ChineseCase::Lower))
        .collect();
    let mut rows: Vec<String> = digits
        .map(|digits| format!("{lower}点{digits}"))
        .into_iter()
        .collect();
    rows.extend(amount_in_capitals(code));
    rows
}

/// 壹佰贰拾叁元整, 壹拾元零伍分, 伍角: the capitals written on invoices and cheques, for a non-negative plain number with at most two decimals below `CHINESE_NUMERAL_LIMIT`.
fn amount_in_capitals(number: &str) -> Option<String> {
    let (integer, fraction) = number.split_once('.').unwrap_or((number, ""));
    if integer.is_empty()
        || fraction.len() > 2
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let yuan = integer
        .parse::<u64>()
        .ok()
        .filter(|value| *value < CHINESE_NUMERAL_LIMIT)?;
    let mut digits = fraction.bytes().map(|digit| digit - b'0');
    let jiao = digits.next().unwrap_or(0);
    let fen = digits.next().unwrap_or(0);
    let mut text = String::new();
    if yuan > 0 || (jiao == 0 && fen == 0) {
        text.push_str(&chinese_integer(yuan, ChineseCase::Upper)?);
        text.push('元');
    }
    if jiao == 0 && fen == 0 {
        text.push('整');
        return Some(text);
    }
    if jiao > 0 {
        text.push_str(&chinese_digit(jiao, ChineseCase::Upper)?);
        text.push('角');
    } else if yuan > 0 {
        text.push('零');
    }
    if fen > 0 {
        text.push_str(&chinese_digit(fen, ChineseCase::Upper)?);
        text.push('分');
    }
    Some(text)
}

/// Capitals spell out the leading one of 10-19 (壹拾肆, not 拾肆), as amounts are written so that nothing can be added in front; everyday numerals drop it (十四).
fn chinese_integer(value: u64, case: ChineseCase) -> Option<String> {
    let text = value
        .to_chinese(
            ChineseVariant::Simple,
            case,
            ChineseCountMethod::TenThousand,
        )
        .ok()?;
    Some(if case == ChineseCase::Upper && text.starts_with('拾') {
        format!("壹{text}")
    } else {
        text
    })
}

fn chinese_digit(digit: u8, case: ChineseCase) -> Option<String> {
    digit.to_chinese_naive(ChineseVariant::Simple, case).ok()
}

/// `YYYY.M.D` naming a real day.
fn parse_date(code: &str) -> Option<Date> {
    let mut parts = code.split('.');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    let digits = |part: &str, lengths: std::ops::RangeInclusive<usize>| {
        lengths.contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    if parts.next().is_some()
        || !digits(year, 4..=4)
        || !digits(month, 1..=2)
        || !digits(day, 1..=2)
    {
        return None;
    }
    let month = Month::try_from(month.parse::<u8>().ok()?).ok()?;
    Date::from_calendar_date(year.parse().ok()?, month, day.parse().ok()?).ok()
}

fn date_rows(date: Date) -> Vec<String> {
    let year = date.year();
    let month = u32::from(u8::from(date.month()));
    let day = u32::from(date.day());
    let weekday = WEEKDAYS[usize::from(date.weekday().number_days_from_sunday())];
    let mut rows = vec![
        format!("{year}年{month}月{day}日"),
        format!("{year:04}-{month:02}-{day:02}"),
        format!("{year}年{month}月{day}日 {weekday}"),
        format!(
            "{}年{}月{}日",
            year_digits(year.unsigned_abs()),
            chinese_number(month),
            chinese_number(day)
        ),
        weekday.to_owned(),
    ];
    // A date the lunar calendar does not cover drops the row, as the date/time mode does.
    rows.extend(lunar_date(&LocalDateTime {
        year,
        month,
        day,
        ..LocalDateTime::default()
    }));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(code: &str) -> Vec<String> {
        query_expression(code)
            .into_iter()
            .map(|row| row.word)
            .collect()
    }

    #[test]
    fn arithmetic_shows_the_result_the_equation_and_the_amount() {
        assert_eq!(words("1+2"), ["3", "1+2=3", "叁元整"]);
        assert_eq!(words("0.1+0.2"), ["0.3", "0.1+0.2=0.3", "叁角"]);
        assert_eq!(words("2*(3+4)"), ["14", "2*(3+4)=14", "壹拾肆元整"]);
        assert_eq!(words("2^10"), ["1024", "2^10=1024", "壹仟零贰拾肆元整"]);
        assert_eq!(words("7%3"), ["1", "7%3=1", "壹元整"]);
        assert_eq!(words("1/3"), ["0.333333333333", "1/3=0.333333333333"]);
        assert_eq!(words("10-12"), ["-2", "10-12=-2"]);
        assert_eq!(words("-(1-1)"), ["0", "-(1-1)=0", "零元整"]);
        assert_eq!(words("10^20"), ["1e20", "10^20=1e20"]);
    }

    #[test]
    fn rows_are_generated_with_descending_weights() {
        let rows = query_expression("1+2");
        for (index, row) in rows.iter().enumerate() {
            assert_eq!(row.source, CandidateSource::Generated);
            assert_eq!(row.weight, (rows.len() - index) as i64);
            assert!(row.pinyin.is_empty());
        }
    }

    #[test]
    fn incomplete_or_undefined_expressions_have_no_rows() {
        for code in [
            "", "1+", "(1+2", "1/0", "0/0", "*3", "1..2", "2^99999", "1+a", "１+2",
        ] {
            assert!(words(code).is_empty(), "{code:?}");
        }
    }

    #[test]
    fn integers_are_written_in_chinese_numerals() {
        assert_eq!(words("123"), ["一百二十三", "壹佰贰拾叁", "壹佰贰拾叁元整"]);
        assert_eq!(words("10"), ["十", "壹拾", "壹拾元整"]);
        assert_eq!(words("14"), ["十四", "壹拾肆", "壹拾肆元整"]);
        assert_eq!(
            words("100200"),
            ["十万零二百", "壹拾万零贰佰", "壹拾万零贰佰元整"]
        );
        assert_eq!(
            words("1010"),
            ["一千零一十", "壹仟零壹拾", "壹仟零壹拾元整"]
        );
        assert_eq!(words("0"), ["零", "零", "零元整"]);
        assert_eq!(words("999999999999999").len(), 3);
        assert!(words("1000000000000000").is_empty());
    }

    #[test]
    fn decimals_read_with_the_point_and_as_amounts() {
        assert_eq!(
            words("123.45"),
            ["一百二十三点四五", "壹佰贰拾叁元肆角伍分"]
        );
        assert_eq!(words("10.05"), ["十点零五", "壹拾元零伍分"]);
        assert_eq!(words("0.5"), ["零点五", "伍角"]);
        assert_eq!(words("12.00"), ["十二点零零", "壹拾贰元整"]);
        assert_eq!(words("3.14159"), ["三点一四一五九"]);
        assert!(words("1.").is_empty());
        assert!(words(".5").is_empty());
    }

    #[test]
    fn dates_read_in_chinese_with_the_weekday_and_the_lunar_date() {
        assert_eq!(
            words("2026.10.1"),
            [
                "2026年10月1日",
                "2026-10-01",
                "2026年10月1日 星期四",
                "二〇二六年十月一日",
                "星期四",
                "丙午年八月二十一日",
            ]
        );
        assert_eq!(words("2024.2.29")[0], "2024年2月29日");
        // Not a day, and not arithmetic either.
        for code in ["2026.2.30", "2026.13.1", "26.1.1", "2026.1.1.1"] {
            assert!(words(code).is_empty(), "{code:?}");
        }
    }

    #[test]
    fn numbers_format_to_twelve_significant_digits() {
        assert_eq!(format_number(0.1 + 0.2).as_deref(), Some("0.3"));
        assert_eq!(format_number(-0.0).as_deref(), Some("0"));
        assert_eq!(format_number(1e-7).as_deref(), Some("1e-7"));
        assert_eq!(
            format_number(123456789012.6).as_deref(),
            Some("123456789013")
        );
        assert_eq!(format_number(f64::INFINITY), None);
        assert_eq!(format_number(f64::NAN), None);
    }
}

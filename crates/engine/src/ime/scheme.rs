//! The active input scheme. Five concrete schemes with the same five operations: an enum, not a trait object.

use crate::japanese::JapaneseRomajiScheme;
use crate::korean::KoreanScheme;
use crate::quanpin::QuanpinScheme;
use crate::shuangpin::profile::profile;
use crate::shuangpin::ShuangpinScheme;
use crate::types::{QueryRequest, SchemeKey, SchemeType, ShuangpinProfileKind};
use crate::wubi::scheme::WubiScheme;

pub enum Scheme {
    Quanpin(QuanpinScheme),
    Shuangpin(ShuangpinScheme),
    Wubi(WubiScheme),
    Japanese(JapaneseRomajiScheme),
    Korean(KoreanScheme),
}

impl Scheme {
    /// ime_session.cpp:371-386; the profile only matters for shuangpin.
    pub fn new(scheme: SchemeType, profile_kind: ShuangpinProfileKind) -> Self {
        match scheme {
            SchemeType::Quanpin => Self::Quanpin(QuanpinScheme::new()),
            SchemeType::Shuangpin => Self::Shuangpin(ShuangpinScheme::new(profile(profile_kind))),
            SchemeType::Wubi => Self::Wubi(WubiScheme::new()),
            SchemeType::JapaneseRomaji => Self::Japanese(JapaneseRomajiScheme::new()),
            SchemeType::Korean => Self::Korean(KoreanScheme::new()),
        }
    }

    pub fn scheme_type(&self) -> SchemeType {
        match self {
            Self::Quanpin(_) => SchemeType::Quanpin,
            Self::Shuangpin(_) => SchemeType::Shuangpin,
            Self::Wubi(_) => SchemeType::Wubi,
            Self::Japanese(_) => SchemeType::JapaneseRomaji,
            Self::Korean(_) => SchemeType::Korean,
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Quanpin(scheme) => scheme.reset(),
            Self::Shuangpin(scheme) => scheme.reset(),
            Self::Wubi(scheme) => scheme.reset(),
            Self::Japanese(scheme) => scheme.reset(),
            Self::Korean(scheme) => scheme.reset(),
        }
    }

    pub fn handle_key(&mut self, key: SchemeKey) {
        match self {
            Self::Quanpin(scheme) => scheme.handle_key(key),
            Self::Shuangpin(scheme) => scheme.handle_key(key),
            Self::Wubi(scheme) => scheme.handle_key(key),
            Self::Japanese(scheme) => scheme.handle_key(key),
            Self::Korean(scheme) => scheme.handle_key(key),
        }
    }

    pub fn build_request(&self) -> QueryRequest {
        match self {
            Self::Quanpin(scheme) => scheme.build_request(),
            Self::Shuangpin(scheme) => scheme.build_request(),
            Self::Wubi(scheme) => scheme.build_request(),
            Self::Japanese(scheme) => scheme.build_request(),
            Self::Korean(scheme) => scheme.build_request(),
        }
    }

    pub fn preedit(&self) -> String {
        match self {
            Self::Quanpin(scheme) => scheme.preedit(),
            Self::Shuangpin(scheme) => scheme.preedit(),
            Self::Wubi(scheme) => scheme.preedit(),
            Self::Japanese(scheme) => scheme.preedit(),
            Self::Korean(scheme) => scheme.preedit(),
        }
    }

    /// Wubi keeps no case, so it only takes the plain letters.
    pub fn set_raw_input(&mut self, raw: &str, raw_with_cases: &str) {
        match self {
            Self::Quanpin(scheme) => scheme.set_raw_input(raw, raw_with_cases),
            Self::Shuangpin(scheme) => scheme.set_raw_input(raw, raw_with_cases),
            Self::Wubi(scheme) => scheme.set_raw_input(raw),
            Self::Japanese(scheme) => scheme.set_raw_input(raw, raw_with_cases),
            Self::Korean(scheme) => scheme.set_raw_input(raw, raw_with_cases),
        }
    }

    pub fn as_wubi_mut(&mut self) -> Option<&mut WubiScheme> {
        match self {
            Self::Wubi(scheme) => Some(scheme),
            _ => None,
        }
    }

    pub fn as_wubi(&self) -> Option<&WubiScheme> {
        match self {
            Self::Wubi(scheme) => Some(scheme),
            _ => None,
        }
    }
}

//! The input engine: composition state, input schemes, dictionary queries, ranking and learning.
//!
//! This crate replaces the C++ MSIME-Engine that used to be fetched from `engine-lock.json` and patched by overlay scripts. It owns the input algorithms; `msime-input-runtime` keeps orchestrating hosts and does not duplicate any of it.

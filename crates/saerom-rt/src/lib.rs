//! 모든 공개 함수는 컴파일러가 내는 코드만 부르는 C ABI 진입점이다.
#![allow(clippy::missing_safety_doc)]

mod ops;
mod text;
mod value;

pub use ops::*;
